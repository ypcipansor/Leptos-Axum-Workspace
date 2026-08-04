import { defineConfig, devices } from '@playwright/test';

const PORT = 3000;
const BASE_URL = `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: './tests',

  // Fail the build if a `test.only` is committed. Without this a single
  // focused test silently disables the rest of the suite on main.
  forbidOnly: !!process.env.CI,

  // Compilation and browser startup are slow; the assertions themselves are not.
  timeout: 60_000,
  expect: { timeout: 10_000 },

  // One retry in CI. A test that only passes on retry still shows up as flaky
  // in the report rather than being silently forgiven.
  retries: process.env.CI ? 1 : 0,

  // Serial in CI: the tests share one database, and parallel workers
  // registering accounts would contend over it.
  workers: process.env.CI ? 1 : undefined,

  reporter: process.env.CI
    ? [['github'], ['html', { open: 'never' }], ['list']]
    : [['list']],

  use: {
    baseURL: BASE_URL,
    // Captured only when a test fails after a retry, so the artefact is small
    // but a real failure is fully debuggable.
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      // Catches layout that only breaks on a narrow viewport -- horizontal
      // page scroll, tables that overflow the body.
      name: 'mobile-chrome',
      use: { ...devices['Pixel 7'] },
    },
    {
      // The suite that proves pages are usable before any wasm loads. Server
      // rendering is the whole point of this architecture; without a
      // JavaScript-disabled project nothing would actually verify it.
      name: 'no-javascript',
      use: { ...devices['Desktop Chrome'], javaScriptEnabled: false },
      testMatch: /no-js\.spec\.ts/,
    },
  ],

  webServer: {
    // Builds and serves the real release-mode application, rather than a
    // development stub. `cargo leptos serve` also produces the wasm bundle and
    // the stylesheet, so there is nothing else to orchestrate.
    command: 'cargo leptos serve',
    cwd: '..',
    url: `${BASE_URL}/health/live`,
    reuseExistingServer: !process.env.CI,
    // A cold Rust build genuinely takes minutes.
    timeout: 600_000,
    stdout: 'pipe',
    stderr: 'pipe',
    env: {
      APP_ENV: 'development',
      DATABASE_URL:
        process.env.DATABASE_URL ??
        'postgres://postgres:postgres@127.0.0.1:5432/app',
    },
  },
});

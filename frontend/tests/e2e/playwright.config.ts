import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  use: {
    baseURL: 'http://127.0.0.1:1420'
  },
  webServer: {
    command: 'trunk serve --address 127.0.0.1 --port 1420',
    cwd: '../..',
    port: 1420,
    reuseExistingServer: !process.env.CI,
    timeout: 180000 // 3 minutes timeout for compilation inside slow CI environments
  }
});

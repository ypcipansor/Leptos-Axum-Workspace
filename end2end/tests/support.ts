import { randomBytes } from 'node:crypto';
import { expect, type Page } from '@playwright/test';

/** Long enough to satisfy the shared password rules in `app-core`. */
export const PASSWORD = 'correct horse battery staple';

/**
 * A username no other test run will collide with.
 *
 * The tests share one database, so a fixed name would make the second run of
 * the suite fail against a leftover account from the first.
 */
export function uniqueUsername(): string {
  return `user_${randomBytes(4).toString('hex')}`;
}

/**
 * Wait until the wasm bundle has hydrated the page.
 *
 * Typing or clicking before hydration finishes is silently lost, so any test
 * that relies on client-side behaviour (live validation, the theme toggle)
 * must wait for the `data-hydrated` marker the app sets on `<body>` first.
 */
export async function waitForHydration(page: Page): Promise<void> {
  await page.locator('body[data-hydrated="true"]').waitFor();
}

/** Register a fresh account and land on the dashboard. */
export async function signUp(page: Page, username: string): Promise<void> {
  await page.goto('/signup');
  await page.getByLabel('Username').fill(username);
  await page.getByLabel('Password').fill(PASSWORD);
  await page.getByRole('button', { name: 'Create account' }).click();
  await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
}

/** Sign in as an existing account. */
export async function signIn(page: Page, username: string): Promise<void> {
  await page.goto('/signin');
  await page.getByLabel('Username').fill(username);
  await page.getByLabel('Password').fill(PASSWORD);
  await page.getByRole('button', { name: 'Sign in' }).click();
}

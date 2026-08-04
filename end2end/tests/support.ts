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

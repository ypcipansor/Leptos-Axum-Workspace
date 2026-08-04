import { expect, test } from '@playwright/test';
import { PASSWORD, uniqueUsername } from './support';

/**
 * Runs with JavaScript disabled.
 *
 * This is the suite that proves the architecture. Everything here would fail
 * outright against the previous client-rendered build, where a page without
 * JavaScript was a blank `<body>` and no form could be submitted at all.
 */
test.describe('without JavaScript', () => {
  test('the sign-in page is fully rendered by the server', async ({ page }) => {
    await page.goto('/signin');

    await expect(page.getByRole('heading', { name: 'Sign in' })).toBeVisible();
    await expect(page.getByLabel('Username')).toBeVisible();
    await expect(page.getByLabel('Password')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();
  });

  test('an account can be created through a plain form post', async ({ page }) => {
    // `<ActionForm>` degrades to an ordinary HTML form, so the whole flow works
    // without a single byte of wasm having executed.
    await page.goto('/signup');

    await page.getByLabel('Username').fill(uniqueUsername());
    await page.getByLabel('Password').fill(PASSWORD);
    await page.getByRole('button', { name: 'Create account' }).click();

    await expect(page).toHaveURL('/');
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('the dashboard redirects an anonymous visitor', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL('/signin');
  });

  test('page content is present in the initial HTML response', async ({ request }) => {
    // Fetched without a browser at all -- this is what a crawler sees.
    const response = await request.get('/signin');
    expect(response.status()).toBe(200);

    const html = await response.text();
    expect(html).toContain('Sign in');
    expect(html).toContain('<form');
    expect(html).toContain('name="username"');
  });
});

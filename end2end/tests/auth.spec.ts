import { expect, test } from '@playwright/test';
import { PASSWORD, signIn, signUp, uniqueUsername } from './support';

test.describe('authentication', () => {
  test('a new account can be created and lands on the dashboard', async ({ page }) => {
    const username = uniqueUsername();

    await page.goto('/signup');
    await expect(page.getByRole('heading', { name: 'Create an account' })).toBeVisible();

    await page.getByLabel('Username').fill(username);
    await page.getByLabel('Password').fill(PASSWORD);
    await page.getByRole('button', { name: 'Create account' }).click();

    await expect(page).toHaveURL('/');
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
    await expect(page.getByText(username)).toBeVisible();
  });

  test('validation messages appear as the user types', async ({ page }) => {
    // These come from the same parser the server runs. The point of the shared
    // crate is that this message and the server's cannot disagree.
    await page.goto('/signup');

    await page.getByLabel('Username').fill('ab');
    await expect(page.getByText(/at least 3 characters/i)).toBeVisible();

    await page.getByLabel('Password').fill('short');
    await expect(page.getByText(/at least 12 characters/i)).toBeVisible();

    await expect(page.getByRole('button', { name: 'Create account' })).toBeDisabled();
  });

  test('the session cookie is not readable from JavaScript', async ({ page, context }) => {
    // The vulnerability this replaces: the token used to live in localStorage,
    // where any injected script could read it.
    await signUp(page, uniqueUsername());

    const cookies = await context.cookies();
    const session = cookies.find((c) => c.name === 'session' || c.name === '__Host-session');

    expect(session, 'no session cookie was set').toBeDefined();
    expect(session!.httpOnly, 'session cookie is readable from JavaScript').toBe(true);

    const visible = await page.evaluate(() => document.cookie);
    expect(visible).not.toContain(session!.value);

    const stored = await page.evaluate(() => JSON.stringify(window.localStorage));
    expect(stored).not.toContain(session!.value);
  });

  test('wrong credentials are rejected with a non-specific message', async ({ page }) => {
    const username = uniqueUsername();
    await signUp(page, username);
    await page.getByRole('button', { name: 'Sign out' }).click();
    await expect(page).toHaveURL('/signin');

    await page.getByLabel('Username').fill(username);
    await page.getByLabel('Password').fill('definitely the wrong one');
    await page.getByRole('button', { name: 'Sign in' }).click();

    const alert = page.getByRole('alert');
    await expect(alert).toContainText('Invalid username or password');
    // Must not reveal whether the account exists.
    await expect(alert).not.toContainText(/no such user|not found/i);
  });

  test('signing out returns to the sign-in page and ends the session', async ({ page }) => {
    await signUp(page, uniqueUsername());

    await page.getByRole('button', { name: 'Sign out' }).click();
    await expect(page).toHaveURL('/signin');

    // The dashboard must not be reachable again by navigating straight to it.
    await page.goto('/');
    await expect(page).toHaveURL('/signin');
  });

  test('an existing account can sign back in', async ({ page }) => {
    const username = uniqueUsername();
    await signUp(page, username);
    await page.getByRole('button', { name: 'Sign out' }).click();

    await signIn(page, username);
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });
});

test.describe('routing', () => {
  test('an unknown URL renders the not-found page', async ({ page }) => {
    const response = await page.goto('/nope');
    expect(response?.status()).toBe(404);
    await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible();
  });

  test('the browser back button works across pages', async ({ page }) => {
    // The previous implementation tracked the current screen in a signal, so
    // there was no history to go back through.
    await page.goto('/signin');
    await page.getByRole('link', { name: 'Create one' }).click();
    await expect(page).toHaveURL('/signup');

    await page.goBack();
    await expect(page).toHaveURL('/signin');
  });

  test('a deep link opens the page directly', async ({ page }) => {
    await page.goto('/signup');
    await expect(page.getByRole('heading', { name: 'Create an account' })).toBeVisible();
  });
});

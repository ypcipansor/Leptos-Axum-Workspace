import { expect, test } from '@playwright/test';
import { PASSWORD, signIn, signUp, uniqueUsername } from './support';

test.describe('session management', () => {
  test('the current device is listed and labelled', async ({ page }) => {
    await signUp(page, uniqueUsername());

    const table = page.getByRole('table', { name: /active sessions/i });
    await expect(table).toBeVisible();
    await expect(table.getByRole('row')).toHaveCount(2); // header + one session
    await expect(page.getByText('This device')).toBeVisible();
  });

  test('a second sign-in appears as a separate session', async ({ browser, page }) => {
    const username = uniqueUsername();
    await signUp(page, username);

    // A separate context is a genuinely separate browser profile, so it gets
    // its own cookie jar and therefore its own session.
    const other = await browser.newContext();
    const otherPage = await other.newPage();
    await signIn(otherPage, username);
    await expect(otherPage.getByRole('heading', { name: 'Dashboard' })).toBeVisible();

    await page.reload();
    await expect(page.getByRole('table').getByRole('row')).toHaveCount(3);

    await other.close();
  });

  test('revoking another session signs that device out', async ({ browser, page }) => {
    const username = uniqueUsername();
    await signUp(page, username);

    const other = await browser.newContext();
    const otherPage = await other.newPage();
    await signIn(otherPage, username);
    await expect(otherPage.getByRole('heading', { name: 'Dashboard' })).toBeVisible();

    await page.reload();
    await page.getByRole('button', { name: 'Revoke' }).first().click();

    // The list shrinks back to just this device.
    await expect(page.getByRole('table').getByRole('row')).toHaveCount(2);

    // And the revoked device is signed out on its very next navigation, with
    // no window in which the already-issued cookie still works.
    await otherPage.goto('/');
    await expect(otherPage).toHaveURL('/signin');

    await other.close();
  });

  test('revoking the current session signs this device out', async ({ page }) => {
    await signUp(page, uniqueUsername());

    await page.getByRole('button', { name: 'Sign out here' }).click();
    await expect(page).toHaveURL('/signin');
  });

  test('the session table scrolls inside itself on a narrow viewport', async ({ page }) => {
    await signUp(page, uniqueUsername());

    const bodyOverflows = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
    );
    expect(bodyOverflows, 'the page scrolls sideways on a phone').toBe(false);
  });
});

test.describe('theme', () => {
  test('the dark mode choice survives a reload', async ({ page }) => {
    await page.goto('/signin');

    const isDark = () =>
      page.evaluate(() => document.documentElement.classList.contains('dark'));

    const before = await isDark();
    await page.getByRole('button', { name: 'Toggle dark mode' }).click();
    expect(await isDark()).toBe(!before);

    await page.reload();
    // Applied by the blocking script in <head>, so it holds from the first
    // paint rather than flashing the wrong theme first.
    expect(await isDark()).toBe(!before);
  });
});

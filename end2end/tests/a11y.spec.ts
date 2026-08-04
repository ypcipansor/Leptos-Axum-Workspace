import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { signUp, uniqueUsername } from './support';

/**
 * Accessibility checks.
 *
 * The previous UI put its inputs outside any `<form>` and handled Enter with a
 * keydown listener, rendered status messages into a plain `<div>` that no
 * screen reader announced, and had no landmarks at all. Automated scanning
 * catches a good share of that class of defect, so it belongs in CI rather
 * than in a manual checklist nobody runs.
 */

const RULES = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'];

async function scan(page: import('@playwright/test').Page) {
  return new AxeBuilder({ page }).withTags(RULES).analyze();
}

test.describe('accessibility', () => {
  test('the sign-in page has no detectable violations', async ({ page }) => {
    await page.goto('/signin');
    const results = await scan(page);
    expect(results.violations).toEqual([]);
  });

  test('the sign-up page has no detectable violations', async ({ page }) => {
    await page.goto('/signup');
    const results = await scan(page);
    expect(results.violations).toEqual([]);
  });

  test('the dashboard has no detectable violations', async ({ page }) => {
    await signUp(page, uniqueUsername());
    const results = await scan(page);
    expect(results.violations).toEqual([]);
  });

  test('the not-found page has no detectable violations', async ({ page }) => {
    await page.goto('/nope');
    const results = await scan(page);
    expect(results.violations).toEqual([]);
  });

  test('dark mode has no detectable contrast violations', async ({ page }) => {
    // Contrast is the failure mode a second theme introduces most often, and
    // it is invisible to anyone testing only the default one.
    await page.goto('/signin');
    await page.getByRole('button', { name: 'Toggle dark mode' }).click();

    const results = await scan(page);
    expect(results.violations).toEqual([]);
  });

  test('the whole sign-up form is reachable by keyboard', async ({ page }) => {
    await page.goto('/signup');

    await page.keyboard.press('Tab');
    await expect(page.getByLabel('Username')).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(page.getByLabel('Password')).toBeFocused();
  });

  test('Enter submits the form natively', async ({ page }) => {
    // Works because these are real `<form>` elements. The previous UI wired
    // Enter to a keydown handler, so it broke wherever that handler was not
    // attached.
    const username = uniqueUsername();
    await page.goto('/signup');

    await page.getByLabel('Username').fill(username);
    await page.getByLabel('Password').fill('correct horse battery staple');
    await page.getByLabel('Password').press('Enter');

    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
  });

  test('an error message is announced to assistive technology', async ({ page }) => {
    await page.goto('/signin');
    await page.getByLabel('Username').fill('nobody-at-all');
    await page.getByLabel('Password').fill('definitely the wrong one');
    await page.getByRole('button', { name: 'Sign in' }).click();

    // role="alert" is what makes a screen reader read this out. Rendering the
    // same text into a plain div, as before, announces nothing.
    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Invalid username or password');
  });

  test('every page has a main landmark and one h1', async ({ page }) => {
    for (const path of ['/signin', '/signup']) {
      await page.goto(path);
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
    }
  });
});

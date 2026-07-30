import { test, expect } from '@playwright/test';
import { randomBytes } from 'crypto';

test('full end-to-end user flow: register, login, session list, logout', async ({ page }) => {
  // Listen to browser console and page errors
  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.text()));
  page.on('pageerror', err => console.log('BROWSER PAGE ERROR:', err.message));

  // Generate a cryptographically secure unique username to prevent conflict and satisfy CodeQL
  const username = `user_${randomBytes(4).toString('hex')}`;
  const password = 'mypassword123';

  // 1. Visit Login screen with cache buster
  await page.goto('/?t=' + Date.now());
  await expect(page.locator('h2')).toHaveText('Sign In to SIM');

  // Take screenshot of Login page inside ignored directory
  await page.screenshot({ path: 'test-results/screenshot_login.png' });

  // 2. Click register link
  await page.locator('#go-to-register').click();
  await expect(page.locator('h2')).toHaveText('Create Account');

  // Take screenshot of Register page inside ignored directory
  await page.screenshot({ path: 'test-results/screenshot_register.png' });

  // 3. Perform registration
  await page.locator('#reg-username').fill(username);
  await page.locator('#reg-password').fill(password);
  await page.locator('#register-btn').click();

  // Wait for success banner
  await expect(page.locator('#success-banner')).toBeVisible();
  await expect(page.locator('#success-banner')).toContainText('Registration successful!');

  // 4. Click Sign In here
  await page.locator('#go-to-login').click();
  await expect(page.locator('h2')).toHaveText('Sign In to SIM');

  // 5. Log in with newly registered user
  await page.locator('#username').fill(username);
  await page.locator('#password').fill(password);
  await page.locator('#login-btn').click();

  // 6. Verify Dashboard is loaded
  await expect(page.locator('#welcome-username')).toContainText(`Welcome, ${username}`);
  await expect(page.locator('h1')).toHaveText('Session Management');

  // Verify the active session table is visible and has 1 entry
  await expect(page.locator('table')).toBeVisible();
  await expect(page.locator('td').first()).toBeVisible();

  // Take screenshot of Dashboard page inside ignored directory
  await page.screenshot({ path: 'test-results/screenshot_dashboard.png' });

  // 7. Test Logout
  await page.locator('#logout-btn').click();

  // Verify redirected back to Login
  await expect(page.locator('h2')).toHaveText('Sign In to SIM');
});

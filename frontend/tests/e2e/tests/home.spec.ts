import { test, expect } from '@playwright/test';

test('renders app heading and template text', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    'Simple Management Information System'
  );
  await expect(page.getByText('Template sistem informasi manajemen sederhana siap pakai.')).toBeVisible();
});

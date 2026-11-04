import { expect, test } from '@playwright/test';
test('scope boundaries preserve callbacks and normal function slot bindings', async ({ page }) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/scope-fixes.html');
  await expect(page.locator('#config')).toHaveText('true:true');
  await expect(page.locator('#some')).toHaveText('true');
  await expect(page.locator('#self')).toHaveText('provided:render');
  await page.locator('#toggle').click();
  await expect(page.locator('#config')).toHaveText('false:true');
  await expect(page.locator('#some')).toHaveText('false');
  expect(errors).toEqual([]);
});

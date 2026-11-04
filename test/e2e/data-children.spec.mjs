import { expect, test } from '@playwright/test';

test('F14 data callbacks retain arrays while renamed component props forward slots', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/data-children.html');
  await expect(page.locator('main span')).toHaveText('child');
  await expect(page.locator('main strong')).toHaveText('forwarded');
  expect(errors).toEqual([]);
});

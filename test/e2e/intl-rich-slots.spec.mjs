import { expect, test } from '@playwright/test';

test('rich translation slots retain closures and context through language changes and unmount', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/intl-rich-slots.html');
  await expect(page.locator('#rich')).toHaveText('Hello prefix:Ada:Ada');
  await expect(page.locator('#mounts')).toHaveText('1');
  await page.locator('#name').fill('Bob');
  await expect(page.locator('#rich')).toHaveText('Hello prefix:Bob:Bob');
  await page.locator('.rich-action').click();
  await expect(page.locator('#clicks')).toHaveText('1');
  await page.locator('#zh').click();
  await expect(page.locator('#rich')).toHaveText('你好 prefix:Bob:Bob');
  await expect(page.locator('#mounts')).toHaveText('2');
  await expect(page.locator('#unmounts')).toHaveText('1');
  await page.locator('#toggle').click();
  await expect(page.locator('.rich-action')).toHaveCount(0);
  await expect(page.locator('#unmounts')).toHaveText('2');
  await page.locator('#toggle').click();
  await expect(page.locator('#rich')).toHaveText('你好 prefix:Bob:Bob');
  await expect(page.locator('#mounts')).toHaveText('3');
  expect(errors).toEqual([]);
});

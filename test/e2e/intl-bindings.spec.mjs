import { expect, test } from '@playwright/test';
test('direct intl alias switches languages while a local t remains an ordinary function', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/intl-bindings.html');
  await expect(page.locator('#translated')).toHaveText('你好');
  await expect(page.locator('#translated')).toHaveAttribute('title', '你好!');
  await expect(page.locator('#params')).toHaveText('你好 Alice');
  await expect(page.locator('#local')).toHaveText('[local]Hello');
  await page.locator('#name').fill('Bob');
  await expect(page.locator('#params')).toHaveText('你好 Bob');
  await page.locator('#en').click();
  await expect(page.locator('#translated')).toHaveText('Hello');
  await expect(page.locator('#translated')).toHaveAttribute('title', 'Hello!');
  await expect(page.locator('#params')).toHaveText('Hello Bob');
  await page.locator('#zh').click();
  await expect(page.locator('#params')).toHaveText('你好 Bob');
  await expect(page.locator('#local')).toHaveText('[local]Hello');
  expect(errors).toEqual([]);
});

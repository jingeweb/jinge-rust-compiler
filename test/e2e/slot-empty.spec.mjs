import { expect, test } from '@playwright/test';
test('empty slot branch mounts a comment and survives replacement and destruction', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/slot-empty.html');
  await expect(page.locator('main')).toHaveCount(1);
  expect(
    await page.locator('main').evaluate((el) => [...el.childNodes].map((node) => node.nodeType)),
  ).toEqual([8]);
  const comment = await page.locator('main').evaluateHandle((el) => el.firstChild);
  await page.locator('#toggle').click();
  await expect(page.locator('main')).toHaveText('visible');
  expect(await comment.evaluate((node) => node.isConnected)).toBe(false);
  await page.locator('#toggle').click();
  await expect(page.locator('main b')).toHaveCount(0);
  expect(
    await page.locator('main').evaluate((el) => [...el.childNodes].map((node) => node.nodeType)),
  ).toEqual([8]);
  expect(errors).toEqual([]);
});

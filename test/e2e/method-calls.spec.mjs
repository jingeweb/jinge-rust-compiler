import { expect, test } from '@playwright/test';
test('M01 native string methods keep their changing receiver in a real browser', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/paths.html');
  await expect(page.locator('#trimmed')).toHaveText('jinge');
  await page.locator('#text').fill(' Bob ');
  await expect(page.locator('#trimmed')).toHaveText('Bob');
  expect(errors).toEqual([]);
});
test('M02 changing only the owner or only the method updates the rendered invocation', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/paths.html');
  await expect(page.locator('#method-service')).toHaveText('Alice');
  await page.locator('#method-owner').click();
  await expect(page.locator('#method-service')).toHaveText('Bob');
  await page.locator('#method-function').click();
  await expect(page.locator('#method-service')).toHaveText('Bob!');
  expect(errors).toEqual([]);
});

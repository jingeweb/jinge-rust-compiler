import { expect, test } from '@playwright/test';
test('D01 multiple initializes before options while select.value stays after options', async ({
  page,
}) => {
  await page.goto('/paths.html');
  for (const id of ['constant', 'reactive']) {
    await expect
      .poll(() =>
        page.locator(`#${id} option:checked`).evaluateAll((options) => options.map((o) => o.value)),
      )
      .toEqual(['shanghai', 'beijing']);
  }
  await expect(page.locator('#single')).toHaveValue('beijing');
  const select = await page.locator('#reactive').elementHandle();
  await page.locator('#toggle').click();
  await expect(page.locator('#reactive')).not.toHaveAttribute('multiple');
  expect(await select.evaluate((el) => el.isConnected)).toBe(true);
  await page.locator('#toggle').click();
  await expect(page.locator('#reactive')).toHaveAttribute('multiple');
});
test('D02 native input updates string length, dynamic indexes and nullable paths', async ({
  page,
}) => {
  await page.goto('/paths.html');
  await expect(page.locator('#length')).toHaveText('5');
  await expect(page.locator('#index')).toHaveText('j');
  await expect(page.locator('#optional')).toHaveText('undefined');
  await page.locator('#text').fill('');
  await expect(page.locator('#length')).toHaveText('0');
  await expect(page.locator('#index')).toHaveText('undefined');
  await page.locator('#text').fill('a😀b');
  await expect(page.locator('#length')).toHaveText('4');
  await page.locator('#text').fill('你好');
  await page.locator('#index-next').click();
  await expect(page.locator('#index')).toHaveText('好');
  await expect(page.locator('#length')).toHaveText('2');
  await page.locator('#user').click();
  await expect(page.locator('#optional')).toHaveText('2');
});

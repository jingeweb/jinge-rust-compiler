import { expect, test } from '@playwright/test';

test('F06 static inert follows native boolean reflection', async ({ page }) => {
  await page.goto('/inert.html');
  for (const id of ['bare', 'yes', 'no', 'unset']) {
    const expected = id === 'bare' || id === 'yes';
    await expect.poll(() => page.locator(`#${id}`).evaluate((el) => el.inert)).toBe(expected);
    expect(await page.locator(`#${id}`).evaluate((el) => el.hasAttribute('inert'))).toBe(expected);
  }
});
test('F06 reactive inert clears false and undefined and controls native focus', async ({
  page,
}) => {
  await page.goto('/inert.html');
  const content = page.locator('#reactive-inert');
  const input = page.locator('#inside');
  await expect(content).not.toHaveAttribute('inert');
  await input.focus();
  await expect(input).toBeFocused();
  await page.locator('#toggle-inert').click();
  await expect.poll(() => content.evaluate((el) => el.inert)).toBe(true);
  await expect(content).toHaveAttribute('inert');
  await input.evaluate((el) => el.focus());
  await expect(input).not.toBeFocused();
  await page.locator('#toggle-inert').click();
  await expect.poll(() => content.evaluate((el) => el.inert)).toBe(false);
  await expect(content).not.toHaveAttribute('inert');
  await input.focus();
  await expect(input).toBeFocused();
  await page.locator('#toggle-inert').click();
  await expect(content).toHaveAttribute('inert');
  await page.locator('#unset-inert').click();
  await expect.poll(() => content.evaluate((el) => el.inert)).toBe(false);
  await expect(content).not.toHaveAttribute('inert');
});

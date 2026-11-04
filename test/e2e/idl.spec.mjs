import { expect, test } from '@playwright/test';

test('F07 static and reactive IDL bindings use native casing and retain readOnly', async ({
  page,
}) => {
  await page.goto('/idl.html');
  async function check(prefix, complete, enabled) {
    const input = page.locator(`#${prefix}-input`);
    const video = page.locator(`#${prefix}-video`);
    await expect
      .poll(() => input.evaluate((el) => [el.autocomplete, el.autofocus, el.readOnly]))
      .toEqual([complete, enabled, enabled]);
    await expect(input).toHaveAttribute('autocomplete', complete);
    expect(
      await input.evaluate((el) => [el.hasAttribute('autofocus'), el.hasAttribute('readonly')]),
    ).toEqual([enabled, enabled]);
    expect(await video.evaluate((el) => [el.autoplay, el.hasAttribute('autoplay')])).toEqual([
      enabled,
      enabled,
    ]);
    expect(
      await input.evaluate((el) =>
        ['autoComplete', 'autoFocus', 'readonly'].some((name) => Object.hasOwn(el, name)),
      ),
    ).toBe(false);
    expect(await video.evaluate((el) => Object.hasOwn(el, 'autoPlay'))).toBe(false);
  }
  await check('static', 'username', true);
  await check('reactive', 'username', true);
  await page.locator('#toggle').click();
  await check('reactive', 'off', false);
  await page.locator('#clear').click();
  await check('reactive', '', true);
});

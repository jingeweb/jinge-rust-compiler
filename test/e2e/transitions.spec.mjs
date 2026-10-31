import { expect, test } from '@playwright/test';
const eventCount = (page, name) =>
  page.evaluate((name) => window.__E2E__.events.filter((x) => x === name).length, name);
async function expectEvent(page, name, count = 1) {
  await expect.poll(() => eventCount(page, name)).toBe(count);
}
async function expectIntermediateOpacity(locator) {
  await expect
    .poll(
      () =>
        locator.evaluate((el) => {
          const value = Number(getComputedStyle(el).opacity);
          return value > 0 && value < 1;
        }),
      { intervals: [10], timeout: 3000 },
    )
    .toBe(true);
}
test.beforeEach(async ({ page }) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.__errors = errors;
  await page.goto('/');
  await expect(page.locator('h1')).toHaveText('Transitions');
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
});
test.afterEach(async ({ page }) => {
  expect(page.__errors).toEqual([]);
});

test('Chrome performs real CSS interpolation and transitionend; retained DOM stays reactive', async ({
  page,
}) => {
  const box = page.locator('#retained-box');
  await box.evaluate((el) => (window.__retained = el));
  await expect(box).toHaveCSS('opacity', '1');
  await page.locator('#toggle').click();
  await expect(box).toHaveClass(/hidden/);
  await expectIntermediateOpacity(box);
  const offset = await box.evaluate((el) => new DOMMatrix(getComputedStyle(el).transform).m41);
  expect(offset).toBeGreaterThan(0);
  expect(offset).toBeLessThan(120);
  await expectEvent(page, 'afterLeave');
  await expect(box).toHaveCSS('opacity', '0');
  expect(await box.evaluate((el) => el === window.__retained)).toBe(true);
  await page.locator('#rename').click();
  await expect(box).toHaveText('b');
  await page.locator('#toggle').click();
  await expectIntermediateOpacity(box);
  await expectEvent(page, 'afterEnter');
  await expect(box).toHaveCSS('opacity', '1');
  const events = await page.evaluate(() =>
    window.__E2E__.nativeEvents.filter(
      (x) => x.id === 'retained-box' && x.type === 'transitionend',
    ),
  );
  expect(events.length).toBeGreaterThanOrEqual(4);
  expect(events.every((e) => e.trusted)).toBe(true);
  expect(events.map((e) => e.property)).toContain('transform');
  const callbacks = await page.evaluate(() =>
    window.__E2E__.events.filter((e) =>
      ['beforeLeave', 'afterLeave', 'beforeEnter', 'afterEnter'].includes(e),
    ),
  );
  expect(callbacks).toEqual(['beforeLeave', 'afterLeave', 'beforeEnter', 'afterEnter']);
});

test('destroyAfterLeave keeps DOM during animation, destroys once, and remounts on enter', async ({
  page,
}) => {
  const original = await page.locator('#destroyed-box').elementHandle();
  await page.locator('#toggle-destroy').click();
  await expectIntermediateOpacity(page.locator('#destroyed-box'));
  await expectEvent(page, 'destroyAfterLeave');
  await expect(page.locator('#destroyed-box')).toHaveCount(0);
  expect(await original.evaluate((el) => el.isConnected)).toBe(false);
  expect(
    await page.evaluate(() => window.__E2E__.unmounts.filter((x) => x === 'destroyed-box').length),
  ).toBe(1);
  expect(await eventCount(page, 'destroyAfterLeave')).toBe(1);
  await page.locator('#toggle-destroy').click();
  await expectIntermediateOpacity(page.locator('#destroyed-box'));
  await expectEvent(page, 'destroyAfterEnter');
  await expect(page.locator('#destroyed-box')).toHaveCSS('opacity', '1');
});

test('appear animates from leave class on initial mount', async ({ page }) => {
  const box = page.locator('#appear-box');
  await expectEvent(page, 'appearBeforeEnter');
  await expectIntermediateOpacity(box);
  await expectEvent(page, 'appearAfterEnter');
  await expect(box).toHaveCSS('opacity', '1');
  expect(
    await page.evaluate(() =>
      window.__E2E__.nativeEvents.some(
        (e) => e.id === 'appear-box' && e.type === 'transitionend' && e.trusted,
      ),
    ),
  ).toBe(true);
});

test('leave to enter cancellation keeps the same element and suppresses afterLeave', async ({
  page,
}) => {
  const box = page.locator('#retained-box');
  await box.evaluate((el) => (window.__retained = el));
  await page.locator('#toggle').click();
  await expectIntermediateOpacity(box);
  await page.locator('#toggle').click();
  await expectEvent(page, 'leaveCancelled');
  await expectEvent(page, 'afterEnter');
  await expect(box).toHaveCSS('opacity', '1');
  expect(await eventCount(page, 'afterLeave')).toBe(0);
  expect(await box.evaluate((el) => el === window.__retained)).toBe(true);
  expect(
    await page.evaluate(() =>
      window.__E2E__.nativeEvents.some(
        (e) => e.id === 'retained-box' && e.type === 'transitioncancel' && e.trusted,
      ),
    ),
  ).toBe(true);
});

test('enter to leave cancellation fires enterCancelled and completes leave', async ({ page }) => {
  const box = page.locator('#retained-box');
  await page.locator('#toggle').click();
  await expectEvent(page, 'afterLeave');
  await page.locator('#toggle').click();
  await expectIntermediateOpacity(box);
  await page.locator('#toggle').click();
  await expectEvent(page, 'enterCancelled');
  await expectEvent(page, 'afterLeave', 2);
  await expect(box).toHaveCSS('opacity', '0');
  expect(await eventCount(page, 'afterEnter')).toBe(0);
});

test('TransitionGroup delays removal until transitionend, animates insertions and preserves keyed DOM', async ({
  page,
}) => {
  await page.locator('#item-1').evaluate((el) => (window.__firstItem = el));
  await page.locator('#remove-item').click();
  await expectIntermediateOpacity(page.locator('#item-2'));
  await expect(page.locator('#item-2')).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      window.__E2E__.nativeEvents.some(
        (e) => e.id === 'item-2' && e.type === 'transitionend' && e.trusted,
      ),
    ),
  ).toBe(true);
  await page.locator('#add-item').click();
  await expectIntermediateOpacity(page.locator('#item-4'));
  await expect(page.locator('#item-4')).toHaveCSS('opacity', '1');
  await page.locator('#reverse').click();
  await expect(page.locator('#group [data-key]')).toHaveText(['4', '3', '1']);
  expect(await page.locator('#item-1').evaluate((el) => el === window.__firstItem)).toBe(true);
});

test('destroying a TransitionGroup during leave immediately removes retained animation nodes', async ({
  page,
}) => {
  await page.locator('#remove-item').click();
  await expectIntermediateOpacity(page.locator('#item-2'));
  await page.locator('#toggle-group').click();
  await expect(page.locator('#group [data-key]')).toHaveCount(0);
  await page.locator('#toggle-group').click();
  await expect(page.locator('#group [data-key]')).toHaveText(['1', '3']);
  await expect(page.locator('#item-2')).toHaveCount(0);
});

test('reactive class binding starts a real CSS keyframe animation and emits trusted animationend', async ({
  page,
}) => {
  const box = page.locator('#keyframe-box');
  await expect(box).toHaveClass('box');
  await page.locator('#animate').click();
  await expect(box).toHaveClass('box pulse');
  await expectIntermediateOpacity(box);
  expect(
    await box.evaluate((el) =>
      el.getAnimations().some((animation) => animation instanceof CSSAnimation),
    ),
  ).toBe(true);
  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__E2E__.nativeEvents.some(
          (e) =>
            e.id === 'keyframe-box' &&
            e.type === 'animationend' &&
            e.property === 'pulse' &&
            e.trusted,
        ),
      ),
    )
    .toBe(true);
  await expect(box).toHaveCSS('opacity', '1');
  await page.locator('#rename').click();
  await expect(box).toHaveText('b');
});

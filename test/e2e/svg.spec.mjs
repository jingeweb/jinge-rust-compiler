import { expect, test } from '@playwright/test';
test('SVG exclusive tags render geometry across component, If, For and slot boundaries', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/svg.html');
  await expect(page.locator('circle')).toHaveCount(5);
  expect(
    await page.locator('circle').evaluateAll((nodes) =>
      nodes.map((node) => ({
        namespace: node.namespaceURI,
        geometry: node instanceof SVGGeometryElement,
        width: node instanceof SVGGeometryElement ? node.getBBox().width : 0,
      })),
    ),
  ).toEqual(
    Array.from({ length: 5 }, () => ({
      namespace: 'http://www.w3.org/2000/svg',
      geometry: true,
      width: 40,
    })),
  );
  expect(await page.locator('foreignObject').evaluate((node) => node.namespaceURI)).toBe(
    'http://www.w3.org/2000/svg',
  );
  expect(await page.locator('#html-child').evaluate((node) => node.namespaceURI)).toBe(
    'http://www.w3.org/1999/xhtml',
  );
  await page.locator('#toggle').click();
  await expect(page.locator('#conditional')).toHaveCount(0);
  await page.locator('#toggle').click();
  await expect(page.locator('#conditional')).toHaveCount(1);
  expect(
    await page.locator('#conditional').evaluate((node) => node instanceof SVGCircleElement),
  ).toBe(true);
  const first = await page.locator('#list circle').first().elementHandle();
  await page.locator('#reverse').click();
  await expect(page.locator('#list circle').first()).toHaveAttribute('data-id', '2');
  expect(
    await first.evaluate((node) => node === document.querySelector('#list circle:last-child')),
  ).toBe(true);
  expect(errors).toEqual([]);
});

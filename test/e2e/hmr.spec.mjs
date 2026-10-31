import { expect, test } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';
const widget = new URL('./app/HmrWidget.tsx', import.meta.url);
test('Vite WebSocket HMR updates compiled components without a page reload or duplicate handlers', async ({
  page,
}) => {
  const original = await readFile(widget, 'utf8');
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  try {
    await page.goto('/');
    await expect(page.locator('#hmr-counter')).toHaveText('Original:0');
    await page.evaluate(() => (window.__hmrToken = 'same-document'));
    await page.locator('#hmr-counter').click();
    await expect(page.locator('#hmr-counter')).toHaveText('Original:1');
    await writeFile(widget, original.replace('Original:', 'Updated:'));
    await expect(page.locator('#hmr-counter')).toHaveText('Updated:0');
    expect(await page.evaluate(() => window.__hmrToken)).toBe('same-document');
    await page.locator('#hmr-counter').click();
    await expect(page.locator('#hmr-counter')).toHaveText('Updated:1');
    expect(errors).toEqual([]);
  } finally {
    await writeFile(widget, original);
  }
});
test('syntax errors show the real Vite overlay; correcting TSX recovers without a page reload', async ({
  page,
}) => {
  const original = await readFile(widget, 'utf8');
  const callbackErrors = [];
  page.on('pageerror', (error) => {
    if (error.message.includes('__hmrUpdate__')) callbackErrors.push(error.message);
  });
  page.on('console', (message) => {
    if (message.type() === 'error' && message.text().includes('__hmrUpdate__'))
      callbackErrors.push(message.text());
  });
  try {
    await page.goto('/');
    await expect(page.locator('#hmr-counter')).toHaveText('Original:0');
    await page.evaluate(() => (window.__hmrToken = 'same-document'));
    await writeFile(widget, 'export function HmrWidget(){return <div>}');
    const overlay = page.locator('vite-error-overlay');
    await expect(overlay).toBeAttached();
    await expect(overlay.locator('.message')).toContainText(/error|Expected|Unexpected/);
    await writeFile(widget, original.replace('Original:', 'Recovered:'));
    await expect(overlay).toHaveCount(0);
    await expect(page.locator('#hmr-counter')).toHaveText('Recovered:0');
    expect(await page.evaluate(() => window.__hmrToken)).toBe('same-document');
    expect(callbackErrors).toEqual([]);
  } finally {
    await writeFile(widget, original);
  }
});

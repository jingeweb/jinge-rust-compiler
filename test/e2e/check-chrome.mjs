import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
const browser = await chromium.launch({
  channel: process.env.PLAYWRIGHT_CHANNEL ?? 'chrome',
  headless: true,
});
try {
  const page = await browser.newPage();
  await page.setContent('<title>Jinge Chrome check</title><p>Chrome works</p>');
  assert.equal(await page.locator('p').innerText(), 'Chrome works');
  console.log(
    JSON.stringify({
      browser: browser.version(),
      title: await page.title(),
      channel: process.env.PLAYWRIGHT_CHANNEL ?? 'chrome',
    }),
  );
} finally {
  await browser.close();
}

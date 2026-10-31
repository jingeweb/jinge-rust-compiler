import { defineConfig } from '@playwright/test';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '../..');
export default defineConfig({
  testDir: import.meta.dirname,
  testMatch: '*.spec.mjs',
  workers: 1,
  fullyParallel: false,
  timeout: 15000,
  expect: { timeout: 5000 },
  forbidOnly: !!process.env.CI,
  retries: 0,
  outputDir: path.join(root, 'coverage/e2e-results'),
  reporter: [
    ['list'],
    ['json', { outputFile: path.join(root, 'coverage/e2e-results.json') }],
    ['html', { outputFolder: path.join(root, 'coverage/e2e-report'), open: 'never' }],
  ],
  use: {
    browserName: 'chromium',
    channel: process.env.PLAYWRIGHT_CHANNEL ?? 'chrome',
    baseURL: 'http://127.0.0.1:5197',
    headless: true,
    viewport: { width: 1000, height: 800 },
    reducedMotion: 'no-preference',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: `"${process.execPath}" "${import.meta.dirname}/server.mjs"`,
    url: 'http://127.0.0.1:5197',
    timeout: 30000,
    reuseExistingServer: false,
  },
});

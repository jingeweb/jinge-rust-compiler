import { spawnSync } from 'node:child_process';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '../..');
for (const args of [
  ['test/run.mjs', '--prepare-only'],
  [
    'node_modules/@playwright/test/cli.js',
    'test',
    '--config',
    'test/e2e/playwright.config.mjs',
    ...process.argv.slice(2),
  ],
]) {
  const result = spawnSync(process.execPath, args, { cwd: root, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

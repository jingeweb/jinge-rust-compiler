import { spawnSync } from 'node:child_process';
import { copyFileSync } from 'node:fs';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
if (process.env.JINGE_SKIP_BUILD !== '1') run('cargo', ['build', '--locked']);
const binary =
  process.platform === 'darwin'
    ? 'libjinge_compiler.dylib'
    : process.platform === 'win32'
      ? 'jinge_compiler.dll'
      : 'libjinge_compiler.so';
copyFileSync(path.join(root, 'target/debug', binary), path.join(root, 'index.node'));
run(path.join(root, 'node_modules/.bin/tsup-node'), [
  'ts/index.ts',
  'ts/cli/intl-extract.ts',
  'ts/cli/intl-compile.ts',
  '--out-dir',
  'dist',
  '--clean',
  '--format',
  'esm',
  '--sourcemap',
]);
run(path.join(root, 'node_modules/.bin/tsup-node'), [
  'test/integration/tools.ts',
  '--out-dir',
  'test/.compiled',
  '--clean',
  '--format',
  'esm',
  '--sourcemap',
]);
if (process.argv.includes('--prepare-only')) process.exit(0);
run(process.execPath, [
  '--test',
  '--test-concurrency=1',
  ...(process.argv.length > 2
    ? process.argv.slice(2)
    : [
        'test/integration/regressions.test.mjs',
        'test/integration/dom-paths.test.mjs',
        'test/integration/modules.test.mjs',
        'test/integration/runtime.test.mjs',
        'test/integration/diagnostics.test.mjs',
        'test/integration/vite.test.mjs',
      ]),
]);

import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const dir = path.join(root, 'coverage');
for (const name of ['rust', 'v8', 'rust-html', 'compiler-ts', 'runtime', 'rust-summary.txt'])
  rmSync(path.join(dir, name), { recursive: true, force: true });
for (const sub of ['rust', 'v8']) mkdirSync(path.join(dir, sub), { recursive: true });
function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    stdio: options.capture ? 'pipe' : 'inherit',
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} failed (${result.status})\n${result.stderr ?? ''}`);
  return result.stdout?.trim();
}
const sysroot = run('rustc', ['--print', 'sysroot'], { capture: true });
const host = run('rustc', ['-vV'], { capture: true }).match(/^host: (.+)$/m)[1];
const llvm = path.join(sysroot, 'lib/rustlib', host, 'bin');
try {
  const build = run(
    'cargo',
    ['rustc', '--locked', '--tests', '--message-format=json', '--', '-Cinstrument-coverage'],
    { capture: true },
  );
  const testBinary = build
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line))
    .find(
      (message) =>
        message.reason === 'compiler-artifact' && message.profile.test && message.executable,
    )?.executable;
  if (!testBinary) throw new Error('Rust test executable was not produced');
  const profileEnv = { ...process.env, LLVM_PROFILE_FILE: path.join(dir, 'rust', '%m-%p.profraw') };
  run(testBinary, [], { env: profileEnv });
  run('cargo', ['rustc', '--locked', '--lib', '--', '-Cinstrument-coverage']);
  run(process.execPath, ['test/run.mjs'], {
    env: {
      ...process.env,
      JINGE_SKIP_BUILD: '1',
      LLVM_PROFILE_FILE: path.join(dir, 'rust', '%m-%p.profraw'),
      NODE_V8_COVERAGE: path.join(dir, 'v8'),
    },
  });
  const profiles = readdirSync(path.join(dir, 'rust'))
    .filter((p) => p.endsWith('.profraw'))
    .map((p) => path.join(dir, 'rust', p));
  if (!profiles.length) throw new Error('No Rust profiles were produced');
  const profile = path.join(dir, 'rust', 'merged.profdata');
  run(path.join(llvm, 'llvm-profdata'), ['merge', '-sparse', ...profiles, '-o', profile]);
  const binary = path.join(
    root,
    'target/debug',
    process.platform === 'darwin'
      ? 'libjinge_compiler.dylib'
      : process.platform === 'win32'
        ? 'jinge_compiler.dll'
        : 'libjinge_compiler.so',
  );
  const args = [
    binary,
    `-object=${testBinary}`,
    `-instr-profile=${profile}`,
    `-ignore-filename-regex=(/registry/|/rustc/|/rustlib/|/test/)`,
  ];
  const summary = run(path.join(llvm, 'llvm-cov'), ['report', ...args], { capture: true });
  writeFileSync(path.join(dir, 'rust-summary.txt'), summary + '\n');
  console.log(summary);
  run(path.join(llvm, 'llvm-cov'), [
    'show',
    ...args,
    '-format=html',
    `-output-dir=${path.join(dir, 'rust-html')}`,
  ]);
  await import('./report-v8.mjs');
} finally {
  // Restore normal development artifacts even if profiling fails.
  run('cargo', ['build', '--locked']);
  const binary =
    process.platform === 'darwin'
      ? 'libjinge_compiler.dylib'
      : process.platform === 'win32'
        ? 'jinge_compiler.dll'
        : 'libjinge_compiler.so';
  copyFileSync(path.join(root, 'target/debug', binary), path.join(root, 'index.node'));
}

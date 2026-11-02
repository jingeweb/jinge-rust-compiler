import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';

const root = path.resolve(import.meta.dirname, '..');
const platformNames = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64', 'win32-x64'];
const json = async (file) => JSON.parse(await readFile(file, 'utf8'));
const saveJson = (file, value) => writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
async function fixture(t, version = '4.2.0') {
  const dir = await mkdtemp(path.join(os.tmpdir(), 'jinge-compiler-release-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  await mkdir(path.join(dir, 'scripts'));
  await copyFile(
    path.join(root, 'scripts/sync-version.mjs'),
    path.join(dir, 'scripts/sync-version.mjs'),
  );
  await saveJson(path.join(dir, 'package.json'), {
    name: 'jinge-compiler',
    version,
    scripts: { version: 'node scripts/sync-version.mjs' },
    optionalDependencies: { 'jinge-compiler-core-old': '0.1.0', unrelated: '1.2.3' },
  });
  for (const name of platformNames) {
    const platform = path.join(dir, 'platform', name);
    await mkdir(platform, { recursive: true });
    await saveJson(path.join(platform, 'package.json'), {
      name: `jinge-compiler-${name}`,
      version: '0.1.0',
      main: 'index.node',
      os: [name.split('-')[0]],
      cpu: [name.split('-')[1]],
    });
  }
  await writeFile(
    path.join(dir, 'pnpm-workspace.yaml'),
    'allowBuilds:\n  bun: true\nminimumReleaseAgeExclude:\n  - unrelated@1.2.3\n  - jinge-compiler-core-old@0.1.0\notherSetting: true\n',
  );
  await writeFile(
    path.join(dir, 'Cargo.toml'),
    '[package]\nname = "jinge-compiler"\nversion = "0.1.0"\n\n[dependencies]\nneon = { version = "1.1.2" }\n',
  );
  await writeFile(
    path.join(dir, 'Cargo.lock'),
    'version = 4\n\n[[package]]\nname = "jinge-compiler"\nversion = "0.1.0"\n\n[[package]]\nname = "neon"\nversion = "1.1.2"\n',
  );
  await writeFile(path.join(dir, 'pnpm-lock.yaml'), '# unchanged until platform packages exist\n');
  return dir;
}
function run(dir, command, args) {
  return execFileSync(command, args, {
    cwd: dir,
    encoding: 'utf8',
    env: {
      ...process.env,
      HUSKY: '0',
      npm_lifecycle_event: undefined,
      npm_config_git_tag_version: undefined,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}
const sync = (dir) => run(dir, process.execPath, ['scripts/sync-version.mjs']);
async function assertVersions(dir, version) {
  const pkg = await json(path.join(dir, 'package.json'));
  assert.equal(pkg.version, version);
  for (const name of platformNames) {
    const manifest = await json(path.join(dir, 'platform', name, 'package.json'));
    assert.equal(manifest.version, version);
    assert.equal(manifest.main, 'index.node');
    assert.equal(pkg.optionalDependencies[manifest.name], version);
    assert.ok(
      (await readFile(path.join(dir, 'pnpm-workspace.yaml'), 'utf8')).includes(
        `  - ${manifest.name}@${version}\n`,
      ),
    );
  }
  assert.match(
    await readFile(path.join(dir, 'Cargo.toml'), 'utf8'),
    new RegExp(`version = "${version.replaceAll('.', '\\.')}"`),
  );
  assert.ok(
    (await readFile(path.join(dir, 'Cargo.lock'), 'utf8')).includes(
      `name = "jinge-compiler"\nversion = "${version}"`,
    ),
  );
}

test('syncs all native versions, optional dependencies, pnpm exceptions and Cargo metadata', async (t) => {
  const dir = await fixture(t);
  sync(dir);
  await assertVersions(dir, '4.2.0');
  const pkg = await json(path.join(dir, 'package.json'));
  assert.equal(pkg.optionalDependencies['jinge-compiler-core-old'], undefined);
  assert.equal(pkg.optionalDependencies.unrelated, '1.2.3');
  const yaml = await readFile(path.join(dir, 'pnpm-workspace.yaml'), 'utf8');
  assert.ok(yaml.includes('allowBuilds:\n  bun: true'));
  assert.ok(yaml.includes('  - unrelated@1.2.3'));
  assert.ok(yaml.includes('otherSetting: true'));
  assert.equal(yaml.includes('jinge-compiler-core-old'), false);
  assert.ok(
    (await readFile(path.join(dir, 'Cargo.toml'), 'utf8')).includes('neon = { version = "1.1.2" }'),
  );
  assert.ok(
    (await readFile(path.join(dir, 'Cargo.lock'), 'utf8')).includes(
      'name = "neon"\nversion = "1.1.2"',
    ),
  );
  assert.equal(
    await readFile(path.join(dir, 'pnpm-lock.yaml'), 'utf8'),
    '# unchanged until platform packages exist\n',
  );
});
test('is idempotent, handles prereleases and creates a missing release-age section', async (t) => {
  const dir = await fixture(t, '4.2.0-beta.1');
  await writeFile(path.join(dir, 'pnpm-workspace.yaml'), 'allowBuilds:\n  bun: true\n');
  sync(dir);
  await assertVersions(dir, '4.2.0-beta.1');
  const files = [
    'package.json',
    'Cargo.toml',
    'Cargo.lock',
    'pnpm-workspace.yaml',
    ...platformNames.map((n) => `platform/${n}/package.json`),
  ];
  const before = await Promise.all(files.map((f) => readFile(path.join(dir, f), 'utf8')));
  sync(dir);
  assert.deepEqual(
    await Promise.all(files.map((f) => readFile(path.join(dir, f), 'utf8'))),
    before,
  );
});
test('rejects a malformed platform manifest before writing any versions', async (t) => {
  const dir = await fixture(t);
  await writeFile(path.join(dir, 'platform/win32-x64/package.json'), '{');
  assert.throws(() => sync(dir));
  assert.equal((await json(path.join(dir, 'platform/darwin-arm64/package.json'))).version, '0.1.0');
  assert.equal(
    (await json(path.join(dir, 'package.json'))).optionalDependencies['jinge-compiler-core-old'],
    '0.1.0',
  );
});
test('npm version patch commits synchronized manifests and creates the matching tag', async (t) => {
  const dir = await fixture(t);
  run(dir, 'git', ['init', '--quiet']);
  run(dir, 'git', ['config', 'user.email', 'release-test@example.invalid']);
  run(dir, 'git', ['config', 'user.name', 'Release Test']);
  run(dir, 'git', ['add', '.']);
  run(dir, 'git', ['commit', '--quiet', '-m', 'fixture']);
  await writeFile(path.join(dir, 'unrelated.txt'), 'leave untracked');
  run(dir, 'npm', ['version', 'patch']);
  await assertVersions(dir, '4.2.1');
  assert.equal(run(dir, 'git', ['describe', '--tags', '--exact-match']).trim(), 'v4.2.1');
  assert.equal(run(dir, 'git', ['status', '--porcelain']).trim(), '?? unrelated.txt');
  const cargoAtTag = run(dir, 'git', ['show', 'v4.2.1:Cargo.toml']);
  assert.ok(cargoAtTag.includes('version = "4.2.1"'));
  assert.equal(
    JSON.parse(run(dir, 'git', ['show', 'v4.2.1:platform/linux-arm64/package.json'])).version,
    '4.2.1',
  );
});
test('npm version --no-git-tag-version syncs successfully without a git repository', async (t) => {
  const dir = await fixture(t);
  run(dir, 'npm', ['version', 'patch', '--no-git-tag-version']);
  await assertVersions(dir, '4.2.1');
});
test('repository manifests and release commands agree on the current version', async () => {
  const pkg = await json(path.join(root, 'package.json'));
  await assertVersions(root, pkg.version);
  assert.equal(pkg.scripts['release:patch'], 'npm version patch');
  assert.equal(pkg.scripts['patch-version'], 'npm version patch');
  assert.equal(pkg.scripts.version, 'node scripts/sync-version.mjs');
  assert.deepEqual(
    Object.keys(pkg.optionalDependencies).sort(),
    platformNames.map((n) => `jinge-compiler-${n}`).sort(),
  );
});

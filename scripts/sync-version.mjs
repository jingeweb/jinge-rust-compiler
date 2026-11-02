/**
 * npm version patch bumps the root version, runs this hook, then commits/tags.
 * Sync native package versions, optional dependencies, the pnpm release-age
 * exclusions and Cargo metadata. Refresh pnpm-lock.yaml after publishing the
 * platform packages: the next versions do not exist in the registry yet.
 * Running this file directly only syncs files; it does not stage or commit.
 */
import { execFileSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const rootPkgPath = path.join(root, 'package.json');
const rootPkg = JSON.parse(await readFile(rootPkgPath, 'utf8'));
const version = rootPkg.version;
const outputs = new Map();
const platforms = [];
const platformDir = path.join(root, 'platform');

// Read and validate every manifest before writing any files.
for (const entry of await readdir(platformDir, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const file = path.join(platformDir, entry.name, 'package.json');
  const pkg = JSON.parse(await readFile(file, 'utf8'));
  if (!pkg.name?.startsWith('jinge-compiler-')) {
    throw new Error(`Unexpected platform package name in ${file}`);
  }
  pkg.version = version;
  platforms.push(pkg.name);
  outputs.set(file, `${JSON.stringify(pkg, null, 2)}\n`);
}
if (platforms.length === 0) throw new Error('No platform packages found');

rootPkg.optionalDependencies ??= {};
for (const name of Object.keys(rootPkg.optionalDependencies)) {
  if (name.startsWith('jinge-compiler-') && !platforms.includes(name)) {
    delete rootPkg.optionalDependencies[name];
  }
}
for (const name of platforms) rootPkg.optionalDependencies[name] = version;
rootPkg.optionalDependencies = Object.fromEntries(
  Object.entries(rootPkg.optionalDependencies).sort(([a], [b]) => a.localeCompare(b)),
);
outputs.set(rootPkgPath, `${JSON.stringify(rootPkg, null, 2)}\n`);

const workspaceFile = path.join(root, 'pnpm-workspace.yaml');
const lines = (await readFile(workspaceFile, 'utf8')).split('\n');
const header = 'minimumReleaseAgeExclude:';
const entries = platforms
  .sort((a, b) => a.localeCompare(b))
  .map((name) => `  - ${name}@${version}`);
const headerIndex = lines.findIndex((line) => line === header);
if (headerIndex < 0) {
  outputs.set(workspaceFile, `${lines.join('\n').trimEnd()}\n${header}\n${entries.join('\n')}\n`);
} else {
  let end = headerIndex + 1;
  while (end < lines.length && /^\s+\S/.test(lines[end])) end++;
  // Preserve unrelated exclusions and workspace settings.
  const unrelated = lines
    .slice(headerIndex + 1, end)
    .filter((line) => !/^\s*-\s*['"]?jinge-compiler-/.test(line));
  outputs.set(
    workspaceFile,
    [...lines.slice(0, headerIndex + 1), ...unrelated, ...entries, ...lines.slice(end)].join('\n'),
  );
}

for (const [name, pattern] of [
  ['Cargo.toml', /(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m],
  ['Cargo.lock', /(\[\[package\]\]\r?\nname = "jinge-compiler"\r?\nversion = ")[^"]+(")/],
]) {
  const file = path.join(root, name);
  const content = await readFile(file, 'utf8');
  if (!pattern.test(content)) throw new Error(`Cannot find compiler package version in ${name}`);
  outputs.set(file, content.replace(pattern, `$1${version}$2`));
}

for (const [file, content] of outputs) {
  if ((await readFile(file, 'utf8')) !== content) await writeFile(file, content);
}

// npm's version commit includes the root manifest automatically; stage the
// synchronized files too, but leave unrelated files and pnpm-lock.yaml alone.
if (
  process.env.npm_lifecycle_event === 'version' &&
  process.env.npm_config_git_tag_version !== 'false' &&
  process.env.npm_config_git_tag_version !== ''
) {
  execFileSync(
    'git',
    ['add', '--', ...[...outputs.keys()].map((file) => path.relative(root, file))],
    {
      cwd: root,
      stdio: 'inherit',
    },
  );
}
console.log(`[jinge-compiler] synced ${platforms.length} platform package(s) to ${version}`);
console.log('[jinge-compiler] 平台包发布后记得刷新 lock：pnpm install --lockfile-only');

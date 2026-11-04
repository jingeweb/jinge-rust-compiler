// Explicit development-only artifact sync. No links, installs, version changes or pnpm-store writes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  copyFile,
  mkdir,
  readFile,
  readdir,
  realpath,
  rename,
  stat,
  writeFile,
} from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..'),
  workspace = path.dirname(root);
const reportDir = path.join(root, 'coverage'),
  jobs = new Map(),
  resolutions = [];
const json = async (file) => JSON.parse(await readFile(file, 'utf8'));
async function packageRoot(entry) {
  let dir = path.dirname(entry);
  while (dir !== path.dirname(dir)) {
    try {
      await stat(path.join(dir, 'package.json'));
      return dir;
    } catch {
      dir = path.dirname(dir);
    }
  }
  throw new Error(`No package root for ${entry}`);
}
for (const repo of ['jinge-demos', 'jinge-antd']) {
  const consumer = path.join(workspace, repo),
    scopes = [path.join(consumer, 'package.json')];
  if (repo === 'jinge-demos') {
    for (const entry of await readdir(path.join(consumer, 'packages'), { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const folder = path.join(consumer, 'packages', entry.name);
      if (entry.name === 'hello-world') {
        for (const child of await readdir(folder, { withFileTypes: true })) {
          if (child.isDirectory()) scopes.push(path.join(folder, child.name, 'package.json'));
        }
      } else scopes.push(path.join(folder, 'package.json'));
    }
  }
  for (const scope of scopes) {
    try {
      await stat(scope);
    } catch {
      continue;
    }
    const req = createRequire(scope),
      runtimeEntry = await realpath(req.resolve('jinge'));
    let router;
    try {
      router = await realpath(req.resolve('jinge-router'));
    } catch (error) {
      if (error.code !== 'MODULE_NOT_FOUND') throw error;
    }
    if (router)
      assert.equal(
        await realpath(createRequire(router).resolve('jinge')),
        runtimeEntry,
        `Router and application must share Jinge: ${scope}`,
      );
    for (const name of ['jinge', 'jinge-compiler']) {
      const entry = await realpath(req.resolve(name)),
        target = await packageRoot(entry),
        source = path.join(workspace, name);
      assert.equal(
        (await json(path.join(source, 'package.json'))).version,
        (await json(path.join(target, 'package.json'))).version,
        `Version mismatch: ${target}`,
      );
      jobs.set(target, { repo, name, source, target });
      resolutions.push({ scope, name, entry, router: router ?? null });
    }
  }
}
// Complete all resolution/version checks before replacing any files.
const files = [];
async function syncFile(job, relative) {
  const from = path.join(job.source, relative),
    to = path.join(job.target, relative),
    backup = path.join(reportDir, 'local-method-backup', job.repo, job.name, relative);
  await stat(from);
  await mkdir(path.dirname(to), { recursive: true });
  await mkdir(path.dirname(backup), { recursive: true });
  try {
    await stat(backup);
  } catch {
    try {
      await copyFile(to, backup);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
  await copyFile(from, `${to}.method-tmp`);
  await rename(`${to}.method-tmp`, to);
  files.push({
    target: to,
    source: from,
    sha256: createHash('sha256')
      .update(await readFile(to))
      .digest('hex'),
  });
}
async function syncDirectory(job, relative) {
  for (const item of await readdir(path.join(job.source, relative), { withFileTypes: true })) {
    const child = path.join(relative, item.name);
    if (item.isDirectory()) await syncDirectory(job, child);
    else if (item.isFile()) await syncFile(job, child);
  }
}
for (const job of jobs.values()) {
  await syncDirectory(job, 'dist');
  await syncDirectory(job, 'types');
  if (job.name === 'jinge-compiler') await syncFile(job, 'index.node');
}
await mkdir(reportDir, { recursive: true });
const report = { resolutions, packages: [...jobs.values()], files };
await writeFile(
  path.join(reportDir, 'local-method-consumers.json'),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(
  JSON.stringify(
    {
      packages: report.packages,
      resolutionCount: resolutions.length,
      fileCount: files.length,
      report: path.join(reportDir, 'local-method-consumers.json'),
    },
    null,
    2,
  ),
);

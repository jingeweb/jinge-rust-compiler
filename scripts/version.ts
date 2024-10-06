import path from 'node:path';
import { readFile, readdir, writeFile } from 'node:fs/promises';

const VER_REG = /"version":\s*"([^"]+)"/;
const PLATFORM_DIR = path.resolve(__dirname, '../platform');

const pkgFile = path.resolve(__dirname, '../package.json');
const pkgCnt = await readFile(pkgFile, 'utf-8');
const newVer = VER_REG.exec(pkgCnt)![1].replace(/\d+$/, (m) => `${parseInt(m) + 1}`);

await writeFile(
  pkgFile,
  pkgCnt
    .replace(VER_REG, `"version": "${newVer}"`)
    .replace(/("jinge-compiler-core-[^"]+"):\s*"[^"]+"/g, (_, m1) => `${m1}: "${newVer}"`),
);

const platforms = (await readdir(PLATFORM_DIR)).filter(
  (d) => d.startsWith('linux') || d.startsWith('macos') || d.startsWith('windows'),
);

for await (const platform of platforms) {
  const pkgFile = path.join(PLATFORM_DIR, platform, 'package.json');
  let pkgCnt = await readFile(pkgFile, 'utf-8');
  pkgCnt = pkgCnt.replace(VER_REG, `"version": "${newVer}"`);
  await writeFile(pkgFile, pkgCnt);
}

console.log(`Updated to new veriosn: ${newVer}`);

import { createHash } from 'node:crypto';
import path from 'node:path';
import { promises as fs } from 'node:fs';

import type { Options } from 'csv-parse/sync';
import { parse } from 'csv-parse/sync';

/**
 * 计算文本的 hash。需要和 packages/swc-plugin/intl.rs 中使用算法一致，当前统一为 sha512().toBase64().slice(0,6)。
 * 如果修改两处都要变更。
 */
export function calcIntlTextKey(defaultMessage: string, filename?: string) {
  const hasher = createHash('sha512');
  hasher.update(defaultMessage);
  if (filename) {
    hasher.update(filename);
  }
  const hash = hasher.digest('base64');
  return hash.slice(0, 6);
}

export async function loopReadDir(dir: string) {
  const files = await fs.readdir(dir);
  const result: string[] = [];
  for await (const f of files) {
    const p = path.join(dir, f);
    const st = await fs.stat(p);
    if (st.isDirectory()) {
      result.push(...(await loopReadDir(p)));
    } else if (st.isFile() && /\.(ts|tsx)$/.test(f) && !f.endsWith('.d.ts')) {
      result.push(p);
    }
  }
  return result;
}

export async function parseCsv(file: string, opts: Options = {}) {
  return parse(await fs.readFile(file, 'utf-8'), {
    columns: true,
    skip_empty_lines: true,
    ...opts,
  });
}

export async function writeCsv(head: string[], rows: Record<string, string>[], file: string) {
  const lines = [
    head.join(','),
    ...rows.map((row) => {
      return head
        .map((k) => {
          let v = row[k];
          if (!v) return '';
          v = v.replaceAll('"', () => '\\"');
          if (v.includes(',')) v = `"${v}"`;
          return v;
        })
        .join(',');
    }),
  ];
  await fs.writeFile(file, lines.join('\n'));
}

export function parseExtractArgvs() {
  const cwd = process.cwd();
  const argvs = process.argv.slice(2);
  const parsed = {
    languages: [] as string[],
    translateCsv: path.resolve(cwd, 'intl/translate.csv'),
    inputDirs: [] as string[],
  };
  for (let i = 0; i < argvs.length; i++) {
    const key = argvs[i];
    if (key === '--lang' || key === '-l') {
      parsed.languages = argvs[i + 1].split(',');
    } else if (key === '--csv' || key === '-c') {
      parsed.translateCsv = path.resolve(cwd, argvs[i + 1]);
    } else {
      parsed.inputDirs.push(path.resolve(cwd, key));
      continue;
    }
    i += 1; // skip value index
  }
  if (!parsed.languages.length) {
    console.error('missing --lang parameter');
    process.exit(-1);
  }
  if (!parsed.inputDirs.length) {
    parsed.inputDirs.push(path.resolve(cwd, 'src'));
  }
  return parsed;
}

export function parseCompileArgvs() {
  const cwd = process.cwd();
  const argvs = process.argv.slice(2);
  const parsed = {
    translateCsv: path.resolve(cwd, 'intl/translate.csv'),
    outputDir: path.resolve(cwd, 'intl'),
  };
  for (let i = 0; i < argvs.length; i++) {
    const key = argvs[i];
    if (key === '--csv' || key === '-c') {
      parsed.translateCsv = path.resolve(cwd, argvs[i + 1]);
    } else if (key === '--output-dir' || key === '-o') {
      parsed.outputDir = path.resolve(cwd, argvs[i + 1]);
    } else {
      continue;
    }
    i += 1; // skip value index
  }

  return parsed;
}

export async function loopMkdir(dir: string) {
  const pdir = path.dirname(dir);
  try {
    const st = await fs.stat(pdir);
    if (!st.isDirectory()) throw new Error(`${pdir} is not directory`);
  } catch (ex) {
    if ((ex as { code: string }).code === 'ENOENT') {
      await loopMkdir(pdir);
    } else {
      throw ex;
    }
  }
  try {
    await fs.mkdir(dir);
  } catch (ex) {
    if ((ex as { code: string }).code !== 'EEXIST') throw ex;
  }
}

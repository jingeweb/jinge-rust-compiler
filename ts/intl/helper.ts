import { createHash } from 'node:crypto';
import path from 'node:path';
import fs from 'node:fs';

import type { Options } from 'csv-parse/sync';
import { parse } from 'csv-parse/sync';

/**
 * 计算文本的 hash。需要和 packages/swc-plugin/intl.rs 中使用算法一致，当前统一为 sha512().toBase64().slice(0,6)。
 * 如果修改两处都要变更。
 */
export function calcKey(defaultMessage: string, filename?: string) {
  const hasher = createHash('sha512');
  hasher.update(defaultMessage);
  if (filename) {
    hasher.update(filename);
  }
  const hash = hasher.digest('base64');
  return hash.slice(0, 6);
}

export function loopReadDir(dir: string) {
  const files = fs.readdirSync(dir);
  const result: string[] = [];
  files.forEach((f) => {
    const p = path.join(dir, f);
    const st = fs.statSync(p);
    if (st.isDirectory()) {
      result.push(...loopReadDir(p));
    } else if (st.isFile() && /\.(ts|tsx)$/.test(f) && !f.endsWith('.d.ts')) {
      result.push(p);
    }
  });
  return result;
}

export function parseCsv(file: string, opts: Options = {}) {
  return parse(fs.readFileSync(file, 'utf-8'), {
    columns: true,
    skip_empty_lines: true,
    ...opts,
  });
}

export function writeCsv(head: string[], rows: Record<string, string>[], file: string) {
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
  fs.writeFileSync(file, lines.join('\n'));
}

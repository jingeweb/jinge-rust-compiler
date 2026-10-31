import ts from '@typescript/typescript6';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, writeFile, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';

import { intlCompile } from '../../ts/intl/compile';
import { intlExtract } from '../../ts/intl/extract';
import { calcIntlTextKey, writeCsv, parseCsv, loopMkdir, loopReadDir } from '../../ts/intl/helper';
const scenario = process.argv[2];
const dir = await mkdtemp(path.join(os.tmpdir(), 'jinge-tools-test-'));
const csv = path.join(dir, 'translate.csv');
if (scenario === 'csv') {
  const row = { id: 'key', file: 'a.tsx', orig: 'hello\nworld', en: 'hi\r\nworld' };
  await writeCsv(Object.keys(row), [row], csv);
  assert.deepEqual(await parseCsv(csv), [row]);
} else if (scenario === 'extract') {
  const sourceDir = path.join(dir, 'source');
  await loopMkdir(path.join(sourceDir, 'nested'));
  await loopMkdir(sourceDir);
  await writeFile(
    path.join(sourceDir, 'nested/a.tsx'),
    `import {t} from 'jinge';export const msg=t('你好\\nworld');`,
  );
  await writeFile(
    path.join(sourceDir, 'b.ts'),
    `import {t} from 'jinge';export const msg=t('Hello {name}',{name:'Alice'},{key:' custom '});`,
  );
  await writeFile(path.join(sourceDir, 'ignored.d.ts'), `declare const x: string;`);
  await writeFile(csv, '');
  await intlExtract({ languages: ['en', 'zh'], srcDirs: [sourceDir], translateFilePath: csv });
  const rows = (await parseCsv(csv)) as unknown as Record<string, string>[];
  assert.equal(rows.length, 2);
  assert.ok(rows.some((row: Record<string, string>) => row.id === 'custom'));
  const translated = rows.map((row: Record<string, string>) => ({ ...row, zh: '翻译' }));
  await writeCsv(['id', 'file', 'orig', 'en', 'zh'], translated, csv);
  await intlExtract({ languages: ['en', 'zh'], srcDirs: [sourceDir], translateFilePath: csv });
  assert.ok(
    ((await parseCsv(csv)) as unknown as Record<string, string>[]).every(
      (row) => row.zh === '翻译',
    ),
  );
  assert.equal((await loopReadDir(sourceDir)).length, 2);
} else if (scenario === 'helper-cli') {
  for (const [script, args] of [
    ['intl-extract', []],
    [
      'intl-extract',
      ['--lang', 'en', '--csv', path.join(dir, 'extract.csv'), '--', path.join(dir, 'missing')],
    ],
    ['intl-compile', ['--csv', path.join(dir, 'missing.csv')]],
  ] as const) {
    const r = spawnSync(process.execPath, [`dist/cli/${script}.js`, ...args], { encoding: 'utf8' });
    if (script === 'intl-extract' && args.length) {
      assert.equal(r.status, 0, r.stderr);
    } else {
      assert.notEqual(r.status, 0, r.stderr);
    }
  }
} else if (scenario === 'empty') {
  await writeFile(csv, 'id,file,orig,en\n');
  await intlCompile({ outputDir: dir, translateCsvFile: csv });
} else if (scenario === 'cli') {
  const r = spawnSync(
    process.execPath,
    ['dist/cli/intl-compile.js', '--csv', path.join(dir, 'missing.csv')],
    { encoding: 'utf8' },
  );
  assert.notEqual(r.status, 0, r.stderr);
} else if (scenario === 'key-trim') {
  const binding = createRequire(import.meta.url)(path.join(process.cwd(), 'index.node'));
  const compile = (source: string, options: { type: number; intl: number }) =>
    binding.transform('fixture.ts', options.type, source, false, options.intl);
  for (const key of [' greeting ', '']) {
    const r = compile(
      `import {t} from 'jinge';export const label=t('hello',{}, {key:${JSON.stringify(key)}});`,
      { type: 1, intl: 2 },
    );
    assert.ok(r.code.includes(JSON.stringify(key.trim() || calcIntlTextKey('hello'))), r.code);
  }
} else {
  const rich = scenario.startsWith('rich');
  const key = rich ? 'a/b+c-title' : 'key';
  const source = path.join(dir, 'source.tsx');
  const text = rich ? 'Hello <B>world</B>' : 'Hello ` \\ {name}\nnext';
  await writeFile(
    source,
    `import {t} from 'jinge';export const msg=t('Hello <B>world</B>',{B:($text)=><b title="$text">{$text}</b>},{key:${JSON.stringify(key)}});`,
  );
  // Isolate compiler tests from the CSV writer regression.
  await writeFile(csv, `id,file,orig,en\n${key},${source},orig,"${text.replaceAll('"', '""')}"\n`);
  await intlCompile({ outputDir: dir, translateCsvFile: csv });
  const code = await readFile(path.join(dir, 'en.tsx'), 'utf8');
  const parsed = ts.transpileModule(code, {
    fileName: 'en.tsx',
    reportDiagnostics: true,
    compilerOptions: { module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.Preserve },
  });
  assert.equal(parsed.diagnostics?.length ?? 0, 0, code);
  if (!rich) {
    const js = ts.transpileModule(code, {
      compilerOptions: { module: ts.ModuleKind.ESNext },
    }).outputText;
    const m = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
    assert.equal(m.default.key({ name: 'Alice' }), 'Hello ` \\ Alice\nnext');
  } else if (scenario === 'rich-binding') {
    assert.ok(code.includes('title="$text"'), code);
    assert.ok(code.includes('props.children'), code);
  }
}

if (scenario.startsWith('rich')) {
  const harnessPath = path.join(process.cwd(), 'test/integration/harness.mjs');
  const h = await import(harnessPath);
  const code = await readFile(path.join(dir, 'en.tsx'), 'utf8');
  const dict = await h.load(code);
  await h.rt.intlInit({ defaultLocale: 'rich-tool', dicts: { 'rich-tool': dict.module.default } });
  const v = await h.mount(
    `import {t} from 'jinge';export function A(){return <>{t('Hello <B>world</B>',{B:x=><b>{x}</b>},{key:'a/b+c-title'})}</>}`,
    { intl: 1 },
  );
  try {
    assert.equal(v.text(), 'Hello world');
    assert.equal(document.querySelector('b')?.title, '$text');
  } finally {
    v.dispose();
  }
}

// 临时国际化审查探针；使用仓库本地 bun 运行，产物写到系统临时目录。
import ts from '@typescript/typescript6';
import { mkdir, writeFile, readFile } from 'node:fs/promises';

import { intlCompile } from '../../ts/intl/compile';
import { writeCsv, parseCsv, calcIntlTextKey } from '../../ts/intl/helper';
const dir = `${(await import('node:os')).tmpdir()}/jinge-compiler-review/intl`;
await mkdir(dir, { recursive: true });
const csv = `${dir}/newline.csv`;
await writeCsv(
  ['id', 'file', 'orig', 'en'],
  [{ id: 'id', file: 'a.tsx', orig: 'hello\nworld', en: 'hi\nworld' }],
  csv,
);
try {
  console.log(JSON.stringify({ name: 'csv_multiline', rows: await parseCsv(csv) }));
} catch (e) {
  console.log(JSON.stringify({ name: 'csv_multiline', error: e.message }));
}
for (const [name, text] of [
  ['backtick', 'Hello ` {name}'],
  ['dollar', 'Price ${oops} {name}'],
  ['newline', 'Hello\n{name}'],
]) {
  const out = `${dir}/${name}`;
  await mkdir(out, { recursive: true });
  const fp = `${out}/translate.csv`;
  // Proper quoting here isolates compileText from the writer issue.
  await writeFile(
    fp,
    `id,file,orig,en\nkey,source.tsx,orig,${JSON.stringify(text).replaceAll('\\n', '\n')}\n`,
  );
  try {
    await intlCompile({ outputDir: out, translateCsvFile: fp });
    const code = await readFile(`${out}/en.tsx`, 'utf8');
    const sf = ts.createSourceFile('en.tsx', code, ts.ScriptTarget.Latest, true);
    let result;
    try {
      const js = ts.transpileModule(code, {
        compilerOptions: { module: ts.ModuleKind.ESNext },
      }).outputText;
      const mod = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
      result = mod.default.key({ name: 'Alice' });
    } catch (e) {
      result = { error: e.message };
    }
    console.log(
      JSON.stringify({
        name: `intl_${name}`,
        code,
        diagnostics: sf.parseDiagnostics.map((d) =>
          ts.flattenDiagnosticMessageText(d.messageText, ' '),
        ),
        result,
      }),
    );
  } catch (e) {
    console.log(JSON.stringify({ name: `intl_${name}`, error: e.message }));
  }
}
for (const key of ['a/b+c', 'greeting-title']) {
  const out = `${dir}/${key.replaceAll('/', '_')}`;
  await mkdir(out, { recursive: true });
  const source = `${out}/source.tsx`;
  await writeFile(
    source,
    `import {t} from 'jinge';export const msg=t('Hello <B>world</B>',{B:(text)=><b>{text}</b>},{key:${JSON.stringify(key)}});`,
  );
  await writeFile(
    `${out}/translate.csv`,
    `id,file,orig,en\n${key},${source},orig,Hello <B>world</B>\n`,
  );
  try {
    await intlCompile({ outputDir: out, translateCsvFile: `${out}/translate.csv` });
    const code = await readFile(`${out}/en.tsx`, 'utf8');
    const sf = ts.createSourceFile('en.tsx', code, ts.ScriptTarget.Latest, true);
    console.log(
      JSON.stringify({
        name: 'intl_rich_key',
        key,
        code,
        diagnostics: sf.parseDiagnostics.map((d) =>
          ts.flattenDiagnosticMessageText(d.messageText, ' '),
        ),
      }),
    );
  } catch (e) {
    console.log(JSON.stringify({ name: 'intl_rich_key', key, error: e.message }));
  }
}
console.log(
  JSON.stringify({
    name: 'sample_base64_keys',
    keys: ['hello', 'world', '你好', 'Welcome', 'Hello <B>world</B>'].map((s) => [
      s,
      calcIntlTextKey(s),
    ]),
  }),
);

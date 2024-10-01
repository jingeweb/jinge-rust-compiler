import { promises as fs, readFileSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { type ExtractMessage, extractKeyAndMessage, parseCsv } from './helper';
const CWD = process.cwd();

const sourceCache = new Map<string, Map<string, ExtractMessage>>();

function extractSource(file: string) {
  const filepath = path.resolve(CWD, file);
  let src = sourceCache.get(filepath);
  if (src) return src;
  src = new Map();
  const cnt = readFileSync(filepath, 'utf-8');
  const srcFile = ts.createSourceFile(file, cnt, ts.ScriptTarget.Latest);
  function walk(node: ts.Node) {
    ts.forEachChild(node, walk);

    const km = extractKeyAndMessage(node, 'compile', srcFile);
    if (!km) return;
    src!.set(km.key, km);
  }
  ts.forEachChild(srcFile, walk);

  sourceCache.set(filepath, src);
  return src;
}

async function compileText(
  lang: string,
  file: string,
  key: string,
  text: string,
  flag: { needImportJNode: boolean; richComponents: string[] },
) {
  const srcFile = ts.createSourceFile(`${key}.tsx`, `<>${text}</>`, ts.ScriptTarget.Latest);
  function err(e?: unknown): never {
    throw new Error(`parse failed for ${lang}: ${key} -> ${text}, ${e || 'unexpected grammar.'}`);
  }

  const stmt = srcFile.statements[0];
  if (!stmt || !ts.isExpressionStatement(stmt)) {
    err();
  }
  const expr = stmt.expression;
  if (!ts.isJsxFragment(expr)) {
    err();
  }
  if (!expr.children.length) {
    return `""`;
  }

  const vars = new Set<string>();
  const tags = new Map<string, string>();
  let hasTag = false;
  let stack = [] as unknown[];
  function varSeg(n: string) {
    return {
      toString: () => (hasTag ? n : `$${n}`),
    };
  }
  function dealTag(tag: string) {
    if (vars.has(tag)) err(`conflict tag name "${tag}"`);

    const srcFile = extractSource(file);
    const keyMsg = srcFile.get(key);
    if (!keyMsg) err(`message not found in source file, ${key}: ${text}, ${file}`);
    // console.log(keyMsg);
    const comp = keyMsg.richComps?.get(tag);
    if (!comp) err(`rich component not found: ${key}: ${text}, ${file}, ${tag}`);
    const compName = `T_${key}_${tag}`;
    if (comp.type === 'fc_with_props') {
      flag.needImportJNode = true;
    }
    tags.set(
      tag,
      `function ${compName}(${comp.type === 'fc_with_props' ? 'props: { children: JNode }' : ''}) { ${comp.expr} }`,
    );
    return compName;
  }
  function walk(node: ts.Node) {
    if (ts.isJsxText(node)) {
      stack.push(node.text);
    } else if (ts.isJsxExpression(node)) {
      const e = node.expression;
      if (!e || !ts.isIdentifier(e)) {
        err('bad variable name');
      }
      const varName = e.text;
      if (tags.has(varName)) err(`conflict var name "${varName}"`);
      vars.add(varName);
      stack.push(varSeg(`{props.${varName}}`));
    } else if (ts.isJsxElement(node)) {
      const tagNode = node.openingElement.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      hasTag = true;
      const compName = dealTag(tagNode.text);
      const parentStack = stack;
      stack = [];
      ts.forEachChild(node, walk);
      parentStack.push(`<${compName}>${stack.join('')}</${compName}>`);
      stack = parentStack;
    } else if (ts.isJsxSelfClosingElement(node)) {
      hasTag = true;
      const tagNode = node.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      const compName = dealTag(tagNode.text);
      stack.push(`<${compName} />`);
    }
  }
  ts.forEachChild(expr, walk);

  if (!vars.size && !hasTag) {
    return `${JSON.stringify(text)}`;
  } else if (!hasTag) {
    return `(props: Record<string, unknown>) => \`${stack.join('')}\``;
  } else {
    flag.richComponents.push(...tags.values());
    flag.richComponents
      .push(`function T_${key}(${vars.size ? 'props: Record<string, JNode>' : ''}) {
  return <>${stack.join('')}</>;
}`);
    return `T_${key}`;
  }
}
export async function intlCompile({
  outputDir,
  translateCsvFile,
}: {
  outputDir: string;
  translateCsvFile: string;
}) {
  const trans = (await parseCsv(translateCsvFile)) as Record<string, string>[];
  if (!trans.length) {
    console.warn('Nothing to compile.');
    return;
  }
  const languages = Object.keys(trans[0]).filter((v) => v !== 'id' && v !== 'orig' && v !== 'file');
  console.info('Will compile languages:', languages, '...');

  const outputs = Object.fromEntries(
    languages.map((l) => [
      l,
      {
        needImportJNode: false,
        richComponents: [],
        rows: [] as string[],
      },
    ]),
  );

  await Promise.all(
    languages.map(async (lang) => {
      const loc = outputs[lang];
      for await (const row of trans) {
        const v = row[lang];
        if (!v) continue;
        const result = await compileText(lang, row.file, row.id, v, loc);
        loc.rows.push(`  ${JSON.stringify(row.id)}: ${result}`);
      }
    }),
  );

  for await (const lang of languages) {
    const loc = outputs[lang];
    const cnt = `${loc.needImportJNode ? 'import type { JNode } from "jinge";\n' : ''}${loc.richComponents.length ? `${loc.richComponents.join('\n')}\n` : ''}export default {\n${loc.rows.join(',\n')}\n}`;

    await fs.writeFile(path.join(outputDir, `${lang}.tsx`), cnt);
  }
}

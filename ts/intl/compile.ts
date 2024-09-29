import { promises as fs } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { parseCsv } from './helper';

// const CWD = process.cwd();

async function compileText(
  lang: string,
  file: string,
  key: string,
  text: string,
  flag: { hasVars: boolean; hasTags: boolean },
) {
  const src = ts.createSourceFile(
    `${key}.tsx`,
    `<>${text}</>`,
    ts.ScriptTarget.ES2022,
    /*setParentNodes */ true,
  );
  function err(e?: unknown): never {
    throw new Error(`parse failed for ${lang}: ${key} -> ${text}, ${e || 'unexpected grammar.'}`);
  }

  const stmt = src.statements[0];
  if (!stmt || !ts.isExpressionStatement(stmt)) {
    err();
  }
  const expr = stmt.expression;
  if (!ts.isJsxFragment(expr)) {
    err();
  }
  if (!expr.children.length) {
    return `() => ""`;
  }

  const vars = new Set<string>();
  const tags = new Map<string, string>();
  let stack = [] as string[];
  function walk(node: ts.Node) {
    if (ts.isJsxText(node)) {
      stack.push(node.text);
    } else if (ts.isJsxExpression(node)) {
      const e = node.expression?.getFullText().trim();
      if (!e) err();
      const vn = e.split('.')[0];

      if (tags.has(vn)) err(`conflict var name "${vn}"`);
      vars.add(vn);
      flag.hasVars = true;
      stack.push(`$\{props.${e}}`);
    } else if (ts.isJsxElement(node)) {
      const tagNode = node.openingElement.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      const tag = tagNode.text;
      if (vars.has(tag)) err(`conflict tag name "${tag}"`);
      tags.set(
        tag,
        `function T_${key}_${tag}(props: { children: unknown }) { return <>{ props.children }</>; }`,
      );
      flag.hasTags = true;
      const parentStack = stack;
      stack = [];
      ts.forEachChild(node, walk);
      const c = `T_${key}_${tag}`;
      parentStack.push(`<${c}>${stack.join('')}</${c}>`);
      stack = parentStack;
    } else if (ts.isJsxSelfClosingElement(node)) {
      const tagNode = node.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      const tag = tagNode.text;
      if (vars.has(tag)) err(`conflict tag name "${tag}"`);
      tags.set(tag, `function T_${key}_${tag}() { return <></>; }`);
      flag.hasTags = true;
      const c = `T_${key}_${tag}`;
      stack.push(`<${c} />`);
    }
  }
  ts.forEachChild(expr, walk);

  if (!vars.size && !tags.size) {
    return `() => ${JSON.stringify(text)}`;
  } else if (!tags.size) {
    return `(props: Record<string, unknown>) => \`${stack.join('')}\``;
  } else {
    return `(() => {
${[...tags.values()].join('\n')}
return function T(${vars.size ? 'props: Record<string, unknown>' : ''}) {
  return <>${stack.join('')}</>;
}
})()`;
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
        hasVars: false,
        hasTags: false,
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
        loc.rows.push(`${JSON.stringify(row.id)}: ${result}`);
      }
    }),
  );

  for await (const lang of languages) {
    const loc = outputs[lang];
    const cnt = `export default {\n${loc.rows.join(',\n')}\n}`;

    await fs.writeFile(path.join(outputDir, `${lang}.tsx`), cnt);
  }
}

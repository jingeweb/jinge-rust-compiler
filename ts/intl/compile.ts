import { promises as fs } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { parseCsv } from './helper';

function compileText(
  lang: string,
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
  const tags = new Set<string>();
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
      stack.push(`$\{${e}}`);
    } else if (ts.isJsxElement(node)) {
      const tagNode = node.openingElement.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      const tag = tagNode.text;
      if (vars.has(tag)) err(`conflict tag name "${tag}"`);
      tags.add(tag);
      flag.hasTags = true;
      const parentStack = stack;
      stack = [];
      ts.forEachChild(node, walk);
      if (stack.length > 0) {
        parentStack.push(`$\{$jg$(${tag}, \`${stack.join('')}\`)}`);
      }
      stack = parentStack;
    }
  }
  ts.forEachChild(expr, walk);

  if (!vars.size && !tags.size) {
    return `() => ${JSON.stringify(text)}`;
  } else {
    return `({ ${[...vars.values(), ...tags.values()].join(',')} }: Record<string, string>) => \`${stack.join('')}\``;
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
  const languages = Object.keys(trans[0]).filter((v) => v !== 'id' && v !== 'orig');
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

  languages.forEach((lang) => {
    const loc = outputs[lang];

    trans.forEach((row) => {
      const v = row[lang];
      if (v) {
        loc.rows.push(`${JSON.stringify(row.id)}: ${compileText(lang, row.id, v, loc)}`);
      }
    });
  });

  for await (const lang of languages) {
    const loc = outputs[lang];
    let cnt = `export default {\n${loc.rows.join(',\n')}\n}`;
    if (loc.hasTags) {
      cnt = `const $jg$ = (src: string, content: string) => src.replace('{:?}', content);
${cnt}`;
    }
    await fs.writeFile(path.join(outputDir, `${lang}.ts`), cnt);
  }
}

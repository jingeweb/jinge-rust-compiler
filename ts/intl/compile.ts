import { promises as fs } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { parseCsv } from './helper';

function uf(s: string) {
  return s.replace(/^./, (m) => m.toUpperCase());
}
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
  function walk(node: ts.Node) {
    if (ts.isJsxExpression(node)) {
      const e = node.expression?.getFullText().trim();
      if (!e) err();
      const vn = e.split('.')[0];

      if (tags.has(vn)) err(`conflict var name "${vn}"`);
      vars.add(vn);
      flag.hasVars = true;
    } else if (ts.isJsxElement(node)) {
      const tagNode = node.openingElement.tagName;
      if (!ts.isIdentifier(tagNode)) err('not tag???');
      const tag = tagNode.text;
      if (vars.has(tag)) err(`conflict tag name "${tag}"`);
      tags.add(tag);
      flag.hasTags = true;
      ts.forEachChild(node, walk);
    }
  }
  ts.forEachChild(expr, walk);

  if (!vars.size && !tags.size) {
    return `() => ${JSON.stringify(text)}`;
  } else {
    if (tags.size) {
      tags.forEach((tag) => {
        text = text
          .replace(new RegExp(`\\<${tag}\\>`, 'g'), `<${uf(tag)}>`)
          .replace(new RegExp(`\\</${tag}\\>`, 'g'), `</${uf(tag)}>`);
      });
    }
    return `(ctx?: Ctx) => {
  const { ${[...vars.values()].join(',')} } = (ctx as Record<string, ReactNode>) ?? {};
${[...tags.values()]
  .map(
    (tag) =>
      `const ${uf(
        tag,
      )} = ({ children }: { children?: ReactNode }) => (ctx?.${tag} as CtxFn)?.(children);`,
  )
  .join('\n')}
  return <>${text}</>;
}`;
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
    if (loc.hasTags || loc.hasVars) {
      cnt = `import { ReactNode } from 'react';
  type CtxFn = (c?: ReactNode) => ReactNode;
  type Ctx = Record<string, ReactNode | CtxFn>;${cnt}`;
    }
    await fs.writeFile(path.join(outputDir, `${lang}.tsx`), cnt);
  }
}

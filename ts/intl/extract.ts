import { readFileSync } from 'node:fs';
import ts from 'typescript';
import { loopReadDir, parseCsv, writeCsv } from './helper';
import { calcKey } from './helper';

type Dict = Record<
  string,
  {
    file: string;
    defaultMessage: string;
  }
>;
function parseFile({ file, filename, dict }: { file: string; filename: string; dict: Dict }) {
  const src = ts.createSourceFile(
    file,
    readFileSync(file, 'utf-8'),
    ts.ScriptTarget.ES2022,
    /*setParentNodes */ true,
  );

  let hasMessage = false;

  function walk(node: ts.Node) {
    ts.forEachChild(node, walk);

    if (
      !ts.isCallExpression(node) ||
      !ts.isIdentifier(node.expression) ||
      node.expression.text !== 't'
    ) {
      return;
    }
    // console.log(node);
    const defaultText = node.arguments.at(0);
    if (!defaultText || !ts.isStringLiteral(defaultText)) return;

    const options = node.arguments.at(2);
    let key = '';
    let isolated = false;
    if (options && ts.isObjectLiteralExpression(options)) {
      options.properties.forEach((prop) => {
        if (!ts.isPropertyAssignment(prop)) return;
        if (!ts.isIdentifier(prop.name)) return;
        if (prop.name.text === 'key') {
          if (ts.isStringLiteral(prop.initializer)) {
            key = prop.initializer.text;
          }
        } else if (prop.name.text === 'isolated') {
          isolated = prop.initializer.kind === ts.SyntaxKind.TrueKeyword;
        }
      });
    }

    const defaultMessage = defaultText.text;
    if (!key) {
      key = isolated ? calcKey(defaultMessage, filename) : calcKey(defaultMessage);
    }
    hasMessage = true;
    dict[key] = {
      file: filename,
      defaultMessage,
    };
  }
  ts.forEachChild(src, walk);

  return hasMessage;
}

export async function intlExtract({
  languages,
  srcDir,
  translateFilePath,
}: {
  languages: string[];
  srcDir: string;
  translateFilePath: string;
}) {
  console.info('Start Extract...\n');
  const files = loopReadDir(srcDir);
  const cwd = process.cwd();
  const dict: Dict = {};
  files.forEach((file) => {
    if (!file.startsWith(cwd)) {
      return;
    }
    if (!/\.ts(x)?$/.test(file)) {
      return;
    }

    /** 注意此处 filename 的获取方式需要和 `/src/intl.rs` 中的算法一致(见该文件注释），如果修改两处都要变更。 */
    const filename = file.slice(cwd.length);
    if (
      parseFile({
        file,
        filename,
        dict,
      })
    ) {
      console.info(filename, '  ...Extracted');
    }
  });

  const trans = parseCsv(translateFilePath) as Record<string, string>[];
  const transDict = Object.fromEntries(trans.map((t) => [t.id, t]));
  const rows: Record<string, string>[] = [];

  Object.entries(dict).forEach(([id, v]) => {
    const row: Record<string, string> = { id, orig: v.defaultMessage };
    const transRow = transDict[id];
    if (transRow) {
      languages.forEach((l) => {
        row[l] = transRow[l];
      });
    }
    if (!row[languages[0]]) {
      row[languages[0]] = v.defaultMessage;
    }
    rows.push(row);
  });
  rows.sort((ra, rb) => {
    return ra.file > rb.file ? -1 : ra.file < rb.file ? 1 : 0;
  });
  writeCsv(['id', 'orig', ...languages], rows, translateFilePath);
  console.info('\nAll Done.');
}

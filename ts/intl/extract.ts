import { promises as fs } from 'node:fs';
import ts from 'typescript';
import { loopReadDir, parseCsv, writeCsv } from './helper';
import { calcIntlTextKey } from './helper';
import path from 'node:path';

type Dict = Record<
  string,
  {
    file: string;
    defaultMessage: string;
  }
>;

const CWD = process.cwd();
async function parseFile({ file, filename, dict }: { file: string; filename: string; dict: Dict }) {
  const src = ts.createSourceFile(
    file,
    await fs.readFile(file, 'utf-8'),
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
      key = isolated ? calcIntlTextKey(defaultMessage, filename) : calcIntlTextKey(defaultMessage);
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
  srcDirs,
  translateFilePath,
}: {
  languages: string[];
  srcDirs: string[];
  translateFilePath: string;
}) {
  console.info('Start Extract...\n');
  const dict: Dict = {};
  const cwd = process.cwd();
  for await (const srcDir of srcDirs) {
    try {
      const st = await fs.stat(srcDir);
      if (!st.isDirectory()) {
        console.warn(`Warining: ${srcDir} is not directory, ignored.`);
        continue;
      }
    } catch (ex) {
      if ((ex as { code: string }).code === 'ENOENT') {
        console.error(`Warining: ${srcDir} not exits, ignored.`);
        continue;
      } else {
        throw ex;
      }
    }
    const files = await loopReadDir(srcDir);
    for await (const file of files) {
      /** 注意此处 filename 的获取方式需要和 `/src/intl.rs` 中的算法一致(见该文件注释），如果修改两处都要变更。 */
      const filename = path.relative(cwd, file);
      if (
        await parseFile({
          file,
          filename,
          dict,
        })
      ) {
        console.info(filename, '  ...Extracted');
      }
    }
  }

  const trans = (await parseCsv(translateFilePath)) as Record<string, string>[];
  const transDict = Object.fromEntries(trans.map((t) => [t.id, t]));
  const rows: Record<string, string>[] = [];

  Object.entries(dict).forEach(([id, v]) => {
    const fp = path.relative(CWD, v.file);
    const row: Record<string, string> = { id, file: fp, orig: v.defaultMessage };
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
  await writeCsv(['id', 'file', 'orig', ...languages], rows, translateFilePath);
  console.info('\nAll Done.');
}

import { createHash } from 'node:crypto';
import path from 'node:path';
import { promises as fs } from 'node:fs';
import ts from 'typescript';
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

export interface ExtractRichComp {
  type: 'jsx' | 'fc';
  expr: string;
}
export interface ExtractMessage {
  key: string;
  defaultMessage: string;
  richComps?: Map<string, ExtractRichComp>;
}
/**
 *
 * @param node AST 节点
 * @param mode 提取模式。extract 模式用于 intl-extract ，提取 t() 函数的 defaultMessage 并计算 key。
 * compile 模式用于 intl-compile，提取带有富文本格式的 t() 函数的 defaultMessage 和 params 参数中的富文本组件，计算 key。
 */
export function extractKeyAndMessage(
  node: ts.Node,
  mode: 'extract' | 'compile',
): ExtractMessage | null {
  if (
    !ts.isCallExpression(node) ||
    !ts.isIdentifier(node.expression) ||
    node.expression.text !== 't'
  ) {
    return null;
  }

  const params = node.arguments.at(1);
  let richComps: Map<string, ExtractRichComp> | undefined = undefined;
  if (mode === 'compile') {
    if (!params || !ts.isObjectLiteralExpression(params)) {
      return null; // compile 模式下，如果没有 params 参数或参数不是 object，忽略。
    }
    for (const prop of params.properties) {
      if (!ts.isPropertyAssignment(prop)) continue;
      if (!ts.isIdentifier(prop.name)) continue;
      const expr = prop.initializer;
      if (ts.isJsxElement(expr) || ts.isJsxFragment(expr)) {
        if (!richComps) richComps = new Map();
        richComps.set(prop.name.text, { type: 'jsx', expr: expr.getFullText() });
      } else if (ts.isFunctionExpression(expr) || ts.isArrowFunction(expr)) {
        const arg0 = expr.parameters.at(0)?.initializer;
        let code = expr.getFullText();
        if (arg0 && ts.isIdentifier(arg0)) {
          code = code.replace(new RegExp(`\\b${arg0.text}\\b`, 'g'), 'props.children');
        }
        if (!richComps) richComps = new Map();
        richComps.set(prop.name.text, { type: 'fc', expr: code });
      }
    }
    if (!richComps?.size) return null;
  }

  const defaultText = node.arguments.at(0);
  if (!defaultText || !ts.isStringLiteral(defaultText)) return null;

  const options = node.arguments.at(2);
  let key = '';
  if (options && ts.isObjectLiteralExpression(options)) {
    for (const prop of options.properties) {
      if (!ts.isPropertyAssignment(prop)) continue;
      if (!ts.isIdentifier(prop.name)) continue;
      if (prop.name.text === 'key') {
        if (ts.isStringLiteral(prop.initializer)) {
          key = prop.initializer.text.trim();
        }
        break;
      }
    }
  }

  const defaultMessage = defaultText.text;
  if (!key) key = calcIntlTextKey(defaultMessage);

  return { key, defaultMessage, richComps };
}

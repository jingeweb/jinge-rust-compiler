import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

export interface TransformOptions {
  sourcemap?: boolean;
}
/** rust binding compiler interface */
export interface JingeCompiler {
  transform(
    filename: string,
    type: number,
    code: string,
    sourcemap: boolean,
    intl: number,
  ): {
    code: string;
    parsedComponents: string;
    map?: string | null;
  };
}

function getBinding() {
  const platform = os.platform();
  const arch = os.arch();
  return `${platform}-${arch}`;
}

/** 仓库内 `cargo build` 产出的本地 binding；发布的 npm 包里不会包含该文件。 */
const LOCAL_BINDING = fileURLToPath(new URL('../index.node', import.meta.url));

export function loadBinding(debug = false) {
  const require = createRequire(import.meta.url);
  if (debug) return require(LOCAL_BINDING) as JingeCompiler;
  // 在 jinge-compiler 仓库内（或依赖 file: 本地包的项目里）优先使用本地构建的 binding，
  // 这样调试编译器时不需要手动替换 npm 上预编译的平台包。
  if (existsSync(LOCAL_BINDING)) {
    return require(LOCAL_BINDING) as JingeCompiler;
  }
  // console.log('will load', getBinding());
  return require(`jinge-compiler-${getBinding()}`) as JingeCompiler;
}

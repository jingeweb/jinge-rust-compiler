import { createRequire } from 'node:module';
import os from 'node:os';
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
export function loadBinding(debug = false) {
  const require = createRequire(import.meta.url);
  if (debug) return require('../index.node') as JingeCompiler;
  // console.log('will load', getBinding());
  return require(`jinge-compiler-core-${getBinding()}`) as JingeCompiler;
}

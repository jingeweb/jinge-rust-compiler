import { createRequire } from 'module';

export interface TransformOptions {
  sourcemap?: boolean;
}
/** rust binding compiler interface */
export interface JingeCompiler {
  transform(
    filename: string,
    code: string,
    sourcemap: boolean,
  ): {
    code: string;
    map?: string;
  };
}

export function loadBinding(debug = true) {
  const require = createRequire(import.meta.url);
  return require(`../index${debug ? '.debug' : ''}.node`) as JingeCompiler;
}

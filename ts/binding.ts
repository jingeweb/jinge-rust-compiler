import { createRequire } from 'module';
import os from 'os';
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

function getBinding() {
  const platform = os.platform();

  const arch = os.arch();
  if (platform === 'darwin') {
    return arch === 'arm64' ? 'macos-aarch64.node' : 'macos-x86_64.node';
  } else if (platform === 'win32') {
    if (arch === 'x64') return 'windows-x86_64.node';
  } else if (platform === 'linux') {
    if (arch === 'x64') return 'linux-x86_64.node';
    else if (arch === 'arm64') return 'linux-aarch64.node';
  }
  throw new Error(`unsupport platform ${platform}-${arch}`);
}
export function loadBinding(debug = true) {
  const require = createRequire(import.meta.url);
  if (debug) return require('../index.debug.node') as JingeCompiler;
  return require(`jinge-compiler-core-${getBinding()}`) as JingeCompiler;
}

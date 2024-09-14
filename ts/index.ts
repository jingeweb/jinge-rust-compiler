import type { JingeCompiler } from './binding.js';
import { loadBinding } from './binding.js';

export * from './vite-plugin.js';

export interface JingeCompilerOptions {
  /**
   * 加载 debug 版本的 rust binding，该参数仅用于本地开发测试 jinge-compiler 时使用。
   */
  loadDebugNativeBinding?: boolean;
}

export function loadCompiler(options: JingeCompilerOptions): JingeCompiler {
  return loadBinding(options.loadDebugNativeBinding);
}

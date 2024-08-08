import type { JingeCompiler } from './binding.js';
import { loadBinding } from './binding.js';

export * from './vite-plugin.js';

export interface JingeCompilerOptions {
  debug?: boolean;
}

export function loadCompiler(options: JingeCompilerOptions): JingeCompiler {
  return loadBinding(options.debug);
}

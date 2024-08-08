import type { Plugin } from 'vite';
import { loadBinding } from './binding.js';
export function JingeCompilerVitePlugin(options?: {
  debug?: boolean;
  sourcemap?: boolean;
}): Plugin[] {
  return [
    {
      name: 'vite:jinge',
      apply: 'build',
      config() {
        return {
          esbuild: false,
        };
      },
      transform(code, id) {
        if (!id.endsWith('.tsx') && !id.endsWith('.ts')) return;
        const binding = loadBinding(options?.debug);
        const output = binding.transform(code, { sourcemap: options?.sourcemap });
        console.log(JSON.stringify(output.code));
        return output;
      },
    },
  ];
}

import type { PluginOption } from 'vite';
import { loadBinding } from './binding.js';
export function JingeCompilerVitePlugin(options?: {
  debug?: boolean;
  sourcemap?: boolean;
}): PluginOption[] {
  return [
    {
      name: 'vite:jinge',
      apply: 'build',
      config() {
        return {
          esbuild: false,
        };
      },
      transform(code: string, id: string) {
        if (!id.endsWith('.tsx') && !id.endsWith('.ts')) return;
        const binding = loadBinding(options?.debug);
        const output = binding.transform(id, code, true);
        return { code: output.code, map: output.map };
      },
    },
  ];
}

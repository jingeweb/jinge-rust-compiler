import type { PluginOption } from 'vite';
import { loadBinding } from './binding.js';

function transform(code: string, id: string, options?: JingeVitePluginOptions) {
  if (!id.endsWith('.tsx') && !id.endsWith('.ts')) return;
  const binding = loadBinding(options?.debug);
  const output = binding.transform(id, code, true);
  return { code: output.code, map: output.map };
}
export interface JingeVitePluginOptions {
  debug?: boolean;
  sourcemap?: boolean;
}

export function jingeVitePlugin(options?: JingeVitePluginOptions): PluginOption {
  return [
    {
      name: 'vite:jinge:build',
      apply: 'build',
      transform(code: string, id: string) {
        return transform(code, id, options);
      },
    },
    {
      name: 'vite:jinge:sereve',
      apply: 'serve',
      config() {
        return {
          esbuild: false,
        };
      },
      transform(code: string, id: string) {
        return transform(code, id, options);
      },
    },
  ];
}

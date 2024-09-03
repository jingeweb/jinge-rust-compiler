import type { PluginOption } from 'vite';
import { loadBinding } from './binding.js';
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';

const DIRNAME =
  typeof __dirname !== 'undefined' ? __dirname : dirname(fileURLToPath(import.meta.url));

export interface JingeVitePluginOptions {
  debug?: boolean;
}

const HMR_RUNTIME_PATH = '/@jinge-hmr-runtime';
const SCRIPT_CODE = `import {  } from "__PATH__";`;

export function jingeVitePlugin(options?: JingeVitePluginOptions): PluginOption {
  let hmrEnabled = false;
  let sourcemapEnabled = false;

  function transform(code: string, id: string) {
    if (!id.endsWith('.tsx') && !id.endsWith('.ts')) return;
    const binding = loadBinding(options?.debug);
    const output = binding.transform(id, code, sourcemapEnabled);
    return { code: output.code, map: output.map };
  }

  return [
    {
      name: 'vite:jinge:build',
      apply: 'build',
      config() {
        return {
          esbuild: false,
        };
      },
      configResolved(config) {
        if (config.build?.sourcemap) sourcemapEnabled = true;
      },
      transform(code: string, id: string) {
        return transform(code, id);
      },
    },
    {
      name: 'vite:jinge:resolve-runtime',
      apply: 'serve',
      enforce: 'pre',
      resolveId: (id) => (id === HMR_RUNTIME_PATH ? id : undefined),
      load: (id) =>
        id === HMR_RUNTIME_PATH
          ? readFile(resolve(DIRNAME, '../hmr-runtime.js'), 'utf-8')
          : undefined,
    },
    {
      name: 'vite:jinge:sereve',
      apply: 'serve',
      configResolved(config) {
        if (config.server.hmr !== false) hmrEnabled = true;
        if (config.build?.sourcemap) sourcemapEnabled = true;
      },
      config() {
        return {
          esbuild: false,
        };
      },
      transformIndexHtml: (_, config) => [
        {
          tag: 'script',
          attrs: { type: 'module' },
          children: SCRIPT_CODE.replace(
            '__PATH__',
            config.server!.config.base + HMR_RUNTIME_PATH.slice(1),
          ),
        },
      ],
      transform(code: string, id: string) {
        const result = transform(code, id);
        if (result && hmrEnabled && id.endsWith('.tsx')) {
          result.code += readFileSync(resolve(DIRNAME, '../hmr-inject.js'));
        }
        return result;
      },
      // handleHotUpdate({ server, file, timestamp, modules }) {
      //   console.log(file, timestamp, modules);
      //   // server.ws.send({ type: 'full-reload' });
      //   return [];
      // },
    },
  ];
}

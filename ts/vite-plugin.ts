import type { PluginOption } from 'vite';
import { loadBinding } from './binding.js';

export interface JingeVitePluginOptions {
  /**
   * 加载 debug 版本的 rust binding，该参数仅用于本地开发测试 jinge-compiler 时使用。
   */
  loadDebugNativeBinding?: boolean;
  /**
   * 默认情况下，`jinge` 库通过 `package.json` 导出的是 `dist/jinge.prod.js`，即生产发布版；但 `jinge` 库还提供了 `jinge/dev` 和 `jinge/source` 的导出，依次是导出 `dist/jinge.dev.js` 以及 `src/index.ts` 源码。
   *
   * 配置 importAlias 为 'source' 则会配置 vite alias，将 `import from 'jinge'` 改为 `import from 'jinge/source'`，这样的配置通常用于 jinge 库本身的研发。
   *
   * 需要说明的是，在 vite serve 模式下，如果没指定 `importAlias` 参数，也会默认使用 `dev`，即从 `jinge/dev` 导入非压缩版本的 `dist/jinge.dev.js`。
   *
   * 除 `jinge` 库外，这个参数还会对 `jinge-router` 库以同样的作用生效。
   */
  importAlias?: 'source' | 'dev';
}

const HMR_RUNTIME_PATH = '/@jinge-hmr-runtime';
const HMR_RUNTIME_CODE = `import { initHmr } from 'jinge';initHmr();`;
function HMR_INJECT_CODE(initHmrId: string, replaceHmr: string) {
  return `

export function __hmrUpdate__() {
  ${replaceHmr}
}
if (import.meta.hot) {
  ${initHmrId}
  import.meta.hot.accept((newModule) => {
    newModule.__hmrUpdate__();
  });
}`;
}

function getAliasConfig(importAlias?: 'source' | 'dev') {
  if (!importAlias) return undefined;
  return {
    optimizeDeps: {
      exclude: ['jinge', 'jinge-router'],
    },
    resolve: {
      alias: [
        { find: 'jinge', replacement: `jinge/${importAlias}` },
        {
          find: 'jinge-router',
          replacement: `jinge-router/${importAlias}`,
        },
      ],
    },
  };
}
export function jingeVitePlugin(options?: JingeVitePluginOptions): PluginOption {
  let hmrEnabled = false;
  let sourcemapEnabled = true;
  let base = '';
  function transform(code: string, id: string) {
    const qi = id.lastIndexOf('?');
    if (qi > 0) id = id.slice(0, qi);
    const type = id.endsWith('.tsx') ? 2 : id.endsWith('.ts') ? 1 : 0;
    if (type === 0) return;
    const binding = loadBinding(options?.loadDebugNativeBinding);
    const result = binding.transform(id, type, code, sourcemapEnabled);
    if (!result.map) result.map = null; // 空字符串转成 null
    return result;
  }

  return [
    {
      name: 'vite:jinge:build',
      apply: 'build',
      enforce: 'pre',
      configResolved(config) {
        if (config.build?.sourcemap) sourcemapEnabled = true;
      },
      config() {
        return getAliasConfig(options?.importAlias);
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
      load: (id) => (id === HMR_RUNTIME_PATH ? HMR_RUNTIME_CODE : undefined),
    },
    {
      name: 'vite:jinge:sereve',
      apply: 'serve',
      configResolved(config) {
        if (config.server.hmr !== false) hmrEnabled = true;
        base = config.base ?? '';
        if (base === '/') base = '';
        else if (base.endsWith('/')) base = base.slice(0, base.length - 1);
      },
      config() {
        return {
          ...getAliasConfig(options?.importAlias ?? 'dev'), // serve 模式默认将 import 别名为 dev，即加载 `dist/jinge.dev.js` 而不是 `dist/jinge.prod.js`
          esbuild: false,
        };
      },
      transformIndexHtml: () => [
        {
          tag: 'script',
          attrs: { type: 'module' },
          children: `import '${base}/@jinge-hmr-runtime';`,
        },
      ],
      transform(code: string, id: string) {
        const result = transform(code, id);
        if (!result || !hmrEnabled || !result.parsedComponents) return result;
        const parsedComponents = result.parsedComponents.split(',');
        // console.log(parsedComponents);
        if (!parsedComponents.length) return result;
        const injectCode: string[] = [];
        const injectCode2: string[] = [];
        parsedComponents.forEach((pc) => {
          const hmrId = JSON.stringify(`${id}::${pc}`);
          injectCode.push(`window.__JINGE_HMR__?.registerFunctionComponent(${pc}, ${hmrId})`);
          injectCode2.push(`  window.__JINGE_HMR__?.replaceComponentInstance(${pc});`);
        });
        result.code += HMR_INJECT_CODE(injectCode.join('\n'), injectCode2.join('\n'));
        return result;
      },
    },
  ];
}

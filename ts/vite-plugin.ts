import type { PluginOption } from 'vite';
import { loadBinding } from './binding.js';

export interface JingeVitePluginOptions {
  /**
   * 加载 debug 版本的 rust binding，该参数仅用于本地开发测试 jinge-compiler 时使用。
   */
  loadDebugNativeBinding?: boolean;
  /**
   * 将 jinge 和 jinge-router 的 import 转换成从源码 import。
   */
  importSource?: boolean;
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
export function jingeVitePlugin(options?: JingeVitePluginOptions): PluginOption {
  let hmrEnabled = false;
  let sourcemapEnabled = true;

  function transform(code: string, id: string) {
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
      },
      config() {
        const importSource = !!options?.importSource;
        return {
          resolve: {
            alias: [
              { find: /^jinge$/, replacement: importSource ? 'jinge/source' : 'jinge/dev' },
              {
                find: /^jinge-router$/,
                replacement: importSource ? 'jinge-router/source' : 'jinge-router/dev',
              },
            ],
          },
          esbuild: false,
        };
      },
      transformIndexHtml: () => [
        {
          tag: 'script',
          attrs: { type: 'module' },
          children: `import '/@jinge-hmr-runtime';`,
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

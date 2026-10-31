import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { build, createServer } from 'vite';

import { jingeVitePlugin } from '../../dist/index.js';
import { root, runtimeRoot } from './harness.mjs';
function config(intl = true) {
  return {
    configFile: false,
    root: path.join(root, 'test/integration/vite-app'),
    logLevel: 'silent',
    plugins: [
      jingeVitePlugin({ loadDebugNativeBinding: true, intl }),
      {
        name: 'fixture-runtime',
        resolveId(id) {
          if (id === 'jinge/dev') return path.join(runtimeRoot, 'dist/jinge.dev.js');
        },
      },
    ],
    resolve: {
      alias: [
        { find: /^jinge(?:\/dev)?$/, replacement: path.join(runtimeRoot, 'dist/jinge.dev.js') },
      ],
    },
  };
}
test('real Vite serve transforms TSX, intl TS, query IDs, HMR and sourcemaps', async () => {
  const server = await createServer({ ...config(), server: { middlewareMode: true } });
  try {
    const result = await server.transformRequest('/App.tsx?test=1');
    assert.match(result.code, /createEle/);
    assert.match(result.code, /registerFunctionComponent\(App/);
    assert.match(result.code, /__hmrUpdate__/);
    assert.ok(result.map.mappings);
    const label = await server.transformRequest('/label.ts');
    assert.match(label.code, /m3HSJL/);
    assert.match(label.code, /hello/);
    const html = await server.transformIndexHtml('/', '<html><head></head><body></body></html>');
    assert.match(html, /@jinge-hmr-runtime/);
  } finally {
    await server.close();
  }
});
test('real Vite build produces executable JS chunks without JSX or HMR', async () => {
  const output = await build({
    ...config(false),
    build: {
      write: false,
      outDir: mkdtempSync(path.join(os.tmpdir(), 'jinge-vite-')),
      minify: false,
      sourcemap: false,
    },
  });
  const chunks = output.output.filter((x) => x.type === 'chunk');
  assert.ok(chunks.length);
  assert.ok(chunks.some((c) => c.code.includes('createEle')));
  for (const chunk of chunks) {
    assert.ok(!chunk.code.includes('__hmrUpdate__'));
    assert.equal(chunk.map, null);
  }
});
test('plugin build respects sourcemap setting and .ts intl dropDefaultText', () => {
  const plugins = jingeVitePlugin({ loadDebugNativeBinding: true, intl: true });
  const plugin = plugins.find((p) => p.name === 'vite:jinge:build');
  plugin.configResolved({ build: { sourcemap: false } });
  const result = plugin.transform(
    "import {t} from 'jinge';export const label=t('hello')",
    '/label.ts?x=1',
  );
  assert.equal(result.map, null);
  assert.match(result.code, /m3HSJL/);
  assert.ok(!result.code.includes('hello'));
});

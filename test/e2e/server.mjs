import path from 'node:path';
import { createServer } from 'vite';

import { jingeVitePlugin } from '../../dist/index.js';
const compilerRoot = path.resolve(import.meta.dirname, '../..');
const runtimeRoot = process.env.JINGE_RUNTIME_ROOT ?? path.resolve(compilerRoot, '../jinge');
const runtime = path.join(runtimeRoot, 'dist/jinge.dev.js');
const server = await createServer({
  optimizeDeps: { noDiscovery: true },
  configFile: false,
  root: path.join(import.meta.dirname, 'app'),
  logLevel: 'warn',
  plugins: [
    jingeVitePlugin({ loadDebugNativeBinding: true, intl: true }),
    {
      name: 'test-runtime',
      resolveId(id) {
        if (id === 'jinge/dev') return runtime;
      },
    },
  ],
  resolve: { alias: [{ find: /^jinge(?:\/dev)?$/, replacement: runtime }] },
  server: {
    host: '127.0.0.1',
    port: 5197,
    strictPort: true,
    fs: { allow: [compilerRoot, runtimeRoot] },
  },
});
await server.listen();
for (const signal of ['SIGTERM', 'SIGINT'])
  process.on(signal, () => {
    void server.close().then(() => process.exit(0));
  });

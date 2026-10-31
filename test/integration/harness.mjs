import assert from 'node:assert/strict';
import { copyFileSync, mkdtempSync } from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const root = path.resolve(import.meta.dirname, '../..');
export const runtimeRoot = process.env.JINGE_RUNTIME_ROOT ?? path.resolve(root, '../jinge');
const require = createRequire(import.meta.url);
const dir = mkdtempSync(path.join(os.tmpdir(), 'jinge-compiler-test-'));
const binary =
  process.platform === 'darwin'
    ? 'libjinge_compiler.dylib'
    : process.platform === 'win32'
      ? 'jinge_compiler.dll'
      : 'libjinge_compiler.so';
// Never fall back to a published platform binding.
copyFileSync(path.join(root, 'target/debug', binary), path.join(dir, 'compiler.node'));
export const binding = require(path.join(dir, 'compiler.node'));
export function compile(source, { type = 2, intl = 0, map = false } = {}) {
  return binding.transform(type === 1 ? 'fixture.ts' : 'fixture.tsx', type, source, map, intl);
}
const { JSDOM } = require('jsdom');
const dom = new JSDOM('<!doctype html><body></body>', { url: 'http://localhost/' });
for (const name of [
  'window',
  'document',
  'Node',
  'Element',
  'HTMLElement',
  'SVGElement',
  'Text',
  'Comment',
  'Event',
  'MouseEvent',
  'DocumentFragment',
])
  globalThis[name] = dom.window[name];
globalThis.OffscreenCanvas = class {};
export const runtimeUrl = pathToFileURL(path.join(runtimeRoot, 'dist/jinge.dev.js')).href;
export const rt = await import(runtimeUrl);
let serial = 0;
export async function load(source, options) {
  const result = compile(source, options);
  const code = result.code.replaceAll(/(['"])jinge\1/g, JSON.stringify(runtimeUrl));
  const module = await import(
    `data:text/javascript;base64,${Buffer.from(code + `\n// ${serial++}`).toString('base64')}`
  );
  return { module, result };
}
export async function mount(source, options = {}) {
  document.body.replaceChildren();
  const { module: m, result } = await load(source, options);
  const host = new rt.ComponentHost();
  options.beforeRender?.(m, rt);
  try {
    const nodes = options.slot
      ? rt.renderSlotFunction(host, m.opts['slot:content'], {})
      : rt.renderFunctionComponent(host, m.A, options.props ?? {});
    document.body.append(...nodes);
    rt.handleRenderDone(host);
  } catch (error) {
    rt.destroyComponent(host);
    throw error;
  }
  return {
    m,
    result,
    host,
    text: () => document.body.textContent,
    dispose: () => {
      rt.destroyComponent(host);
      document.body.replaceChildren();
    },
  };
}
export const flush = () => new Promise((resolve) => setTimeout(resolve, 35));
export async function behavior(source, before, mutate, after, options) {
  const view = await mount(source, options);
  try {
    assert.equal(view.text(), before);
    if (mutate) {
      mutate(view.m);
      await flush();
      assert.equal(view.text(), after);
    }
  } finally {
    view.dispose();
  }
}

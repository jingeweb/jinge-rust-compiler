import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { compile, flush, load, mount, rt, root } from './harness.mjs';

test('intl aliases update text, attributes, compound expressions and parameters on locale changes', async () => {
  await rt.intlInit({
    defaultLocale: 'binding-en',
    dicts: {
      'binding-en': { NhX4DJ: 'Hello', greet: (p) => `Hello ${p.name}` },
      'binding-zh': { NhX4DJ: '你好', greet: (p) => `你好 ${p.name}` },
    },
  });
  const v = await mount(
    `import {t as tr,vm} from 'jinge';export const state=vm({name:'Alice'});export function A(){return <><p id="normal" title={tr('Hello')+'!'}>{tr('Hello')}</p><p id="params">{tr('Hello {name}',{name:state.name},{key:'greet'})}</p></>}`,
    { intl: 1 },
  );
  try {
    assert.equal(document.querySelector('#normal').title, 'Hello!');
    assert.equal(v.text(), 'HelloHello Alice');
    v.m.state.name = 'Bob';
    await flush();
    assert.equal(v.text(), 'HelloHello Bob');
    await rt.intlSetLocale('binding-zh');
    await flush();
    assert.equal(document.querySelector('#normal').title, '你好!');
    assert.equal(v.text(), '你好你好 Bob');
  } finally {
    v.dispose();
  }
});
test('local t and shadowed import aliases execute their own functions', async () => {
  const v = await mount(
    `import {t as tr} from 'jinge';function t(text){return '[local]'+text}function Child(tr){return tr('Hello')}export function A(){return <p>{t('Hello')}:{Child(text=>'[shadow]'+text)}</p>}`,
    { intl: 1 },
  );
  try {
    assert.equal(v.text(), '[local]Hello:[shadow]Hello');
  } finally {
    v.dispose();
  }
});
test('plain TS aliased translations use the same key protocol', async () => {
  await rt.intlInit({ defaultLocale: 'binding-ts', dicts: { 'binding-ts': { NhX4DJ: '你好' } } });
  const { module } = await load(
    `import {t as tr} from 'jinge';export const value=tr('Hello');function t(text){return '[local]'+text}export const local=t('Hello');`,
    { type: 1, intl: 1 },
  );
  assert.equal(module.value, '你好');
  assert.equal(module.local, '[local]Hello');
});
test('TS extraction and rich compilation recognize the same direct import bindings', () => {
  const result = spawnSync(process.execPath, [`${root}/test/.compiled/tools.js`, 'intl-bindings'], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
});
for (const [name, source] of [
  ['reexport', `import {t} from './a';export function A(){return <p>{t('Hello')}</p>}`],
  [
    'secondary alias',
    `import {t} from 'jinge';const tr=t;export function A(){return <p>{tr('Hello')}</p>}`,
  ],
])
  test(`intl unsupported binding stays ordinary: ${name}`, () =>
    assert.ok(!compile(source, { intl: 1 }).code.includes('NhX4DJ')));

test('intl-only component roots retain props snapshot diagnostics', () => {
  const source = `import {t as tr} from 'jinge';export function A(props){const name=props.name;return tr('Hello {name}',{name},{key:'greet'})}`;
  const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /局部变量 name.*失去.*响应/);
});

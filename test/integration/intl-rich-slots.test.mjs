import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';

import { compile, flush, load, mount, rt, root, runtimeUrl, runtimeRoot } from './harness.mjs';

function dictionary(source, messages) {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'jinge-rich-slots-'));
  const file = path.join(dir, 'source.tsx');
  writeFileSync(file, source);
  const quote = (value) => `"${String(value).replaceAll('"', '""')}"`;
  const rows = [
    ['id', 'file', 'orig', 'en', 'zh'],
    ...messages.map(([key, en, zh = en]) => [key, file, en, en, zh]),
  ];
  const csv = path.join(dir, 'translate.csv');
  writeFileSync(csv, rows.map((row) => row.map(quote).join(',')).join('\n'));
  const result = spawnSync(
    process.execPath,
    [`${root}/dist/cli/intl-compile.js`, '--csv', csv, '-o', dir],
    { encoding: 'utf8' },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return Object.fromEntries(
    ['en', 'zh'].map((lang) => [lang, readFileSync(path.join(dir, `${lang}.tsx`), 'utf8')]),
  );
}

async function init(code, locale) {
  const en = await load(code.en);
  const zh = await load(code.zh);
  await rt.intlInit({
    defaultLocale: `${locale}-en`,
    dicts: { [`${locale}-en`]: en.module.default, [`${locale}-zh`]: zh.module.default },
  });
}

test('S03 callbacks keep outer values, reactive reads and event closures at their call site', async () => {
  const source = `import {t,vm} from 'jinge';export const state=vm({name:'Ada',clicks:0});export function A(){const prefix='Hello ';return <>{t('A <B>{name}</B>',{name:state.name,B:child=><button on:click={()=>state.clicks++}>{prefix}{child}:{state.clicks}</button>},{key:'rich'})}</>}`;
  const code = dictionary(source, [['rich', 'A <B>{name}</B>', '<B>{name}</B>！']]);
  assert.ok(
    !code.en.includes('clicks') && !code.en.includes('prefix') && !code.en.includes('<button'),
  );
  await init(code, 'capture');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'A Hello Ada:0');
    document.querySelector('button').click();
    await flush();
    assert.equal(view.text(), 'A Hello Ada:1');
    view.m.state.name = 'Bob';
    await flush();
    assert.equal(view.text(), 'A Hello Bob:1');
    const old = document.querySelector('button');
    await rt.intlSetLocale('capture-zh');
    await flush();
    assert.equal(view.text(), 'Hello Bob:1！');
    assert.equal(old.isConnected, false);
    document.querySelector('button').click();
    await flush();
    assert.equal(view.text(), 'Hello Bob:2！');
  } finally {
    view.dispose();
  }
});

test('S03 one dictionary key accepts different renderers at different call sites', async () => {
  const source = `import {t} from 'jinge';export function A(){const first='first';const second='second';return <>{t('X <B>child</B>',{B:x=><b>{first}:{x}</b>},{key:'same'})}{t('X <B>child</B>',{B:x=><i>{second}:{x}</i>},{key:'same'})}</>}`;
  await init(dictionary(source, [['same', 'X <B>child</B>']]), 'sites');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'X first:childX second:child');
    assert.equal(document.querySelectorAll('b').length, 1);
    assert.equal(document.querySelectorAll('i').length, 1);
  } finally {
    view.dispose();
  }
});

test('S03 repeated and nested tags create independent nodes and keep children reactive', async () => {
  const source = `import {t,vm} from 'jinge';export const state=vm({name:'Ada'});export function A(){return <>{t('<B><I>{name}</I></B><B>again</B>',{name:state.name,B:x=><b>{x}</b>,I:x=><i>{x}</i>},{key:'nested'})}</>}`;
  await init(dictionary(source, [['nested', '<B><I>{name}</I></B><B>again</B>']]), 'nested');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'Adaagain');
    const bs = document.querySelectorAll('b');
    assert.equal(bs.length, 2);
    assert.notEqual(bs[0], bs[1]);
    assert.equal(bs[0].querySelector('i').textContent, 'Ada');
    view.m.state.name = 'Bob';
    await flush();
    assert.equal(view.text(), 'Bobagain');
  } finally {
    view.dispose();
  }
});

test('S03 direct JSX and callbacks with no argument keep their lexical environment', async () => {
  const source = `import {t,vm} from 'jinge';export const state=vm({name:'Ada'});export function A(){const prefix='Hi';return <>{t('<X/><Y/>',{X:<b>{prefix}:{state.name}</b>,Y:()=>{const suffix='!';return <i>{prefix}{suffix}</i>}},{key:'jsx'})}</>}`;
  await init(dictionary(source, [['jsx', '<X/><Y/>']]), 'jsx');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'Hi:AdaHi!');
    view.m.state.name = 'Bob';
    await flush();
    assert.equal(view.text(), 'Hi:BobHi!');
  } finally {
    view.dispose();
  }
});

test('S03 normal named callbacks retain locals and their own name', async () => {
  const source = `import {t} from 'jinge';export function A(){const suffix='!';return <>{t('<B>child</B>',{B:function render(child){const prefix=render.name;return <b>{prefix}:{child}{suffix}</b>}},{key:'function'})}</>}`;
  await init(dictionary(source, [['function', '<B>child</B>']]), 'function');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'render:child!');
  } finally {
    view.dispose();
  }
});

test('S03 imported components inherit context and release their hooks on locale change and unmount', async () => {
  const child = compile(
    `import {context,onMount,onUnmount} from 'jinge';export function Button(props){const state=context('intl-test');onMount(()=>state.mounts++);onUnmount(()=>state.unmounts++);return <button on:click={()=>state.clicks++}>{props.children}:{state.name}</button>}`,
  ).code.replaceAll(/(['"])jinge\1/g, JSON.stringify(runtimeUrl));
  const url = `data:text/javascript;base64,${Buffer.from(child).toString('base64')}`;
  const source = `import {t,vm,context} from 'jinge';import {Button} from ${JSON.stringify(url)};export const state=vm({name:'Ada',mounts:0,unmounts:0,clicks:0});export function A(){context('intl-test',state);return <>{t('Hello <B>world</B>',{B:child=><Button>{child}</Button>},{key:'context'})}</>}`;
  await init(dictionary(source, [['context', 'Hello <B>world</B>', '<B>世界</B>']]), 'context');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'Hello world:Ada');
    assert.equal(view.m.state.mounts, 1);
    const old = document.querySelector('button');
    await rt.intlSetLocale('context-zh');
    await flush();
    assert.equal(view.text(), '世界:Ada');
    assert.equal(view.m.state.mounts, 2);
    assert.equal(view.m.state.unmounts, 1);
    assert.equal(old.isConnected, false);
    view.m.state.name = 'Bob';
    await flush();
    assert.equal(old.textContent, 'world:Ada');
    assert.equal(view.text(), '世界:Bob');
    document.querySelector('button').click();
    await flush();
    assert.equal(view.m.state.clicks, 1);
  } finally {
    view.dispose();
  }
  assert.equal(view.m.state.unmounts, 2);
  view.m.state.name = 'removed';
  await flush();
  assert.equal(document.body.textContent, '');
});

test('S03 language output is independent of callback implementation and allows rich-only extraction', () => {
  const source = `import {t} from 'jinge';export const msg=t('Hello <B>world</B>',{B:text=><b>{text}</b>},{key:'extract-rich'});`;
  const dir = mkdtempSync(path.join(os.tmpdir(), 'jinge-extract-rich-'));
  writeFileSync(path.join(dir, 'source.tsx'), source);
  const csv = path.join(dir, 'messages.csv');
  writeFileSync(csv, '');
  const result = spawnSync(
    process.execPath,
    [`${root}/dist/cli/intl-extract.js`, '--lang', 'en', '--csv', csv, dir],
    { encoding: 'utf8' },
  );
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.ok(readFileSync(csv, 'utf8').includes('extract-rich'));
  const first = dictionary(source, [['extract-rich', 'Hello <B>world</B>']]);
  const second = dictionary(source.replace('<b>{text}</b>', '<i>{text}</i>'), [
    ['extract-rich', 'Hello <B>world</B>'],
  ]);
  assert.equal(first.en, second.en);
  const generated = path.join(dir, 'dictionary.tsx');
  writeFileSync(generated, first.en);
  const config = path.join(dir, 'tsconfig.json');
  writeFileSync(
    config,
    JSON.stringify({
      compilerOptions: {
        target: 'es2025',
        module: 'ESNext',
        moduleResolution: 'Bundler',
        lib: ['ESNext', 'DOM'],
        strict: true,
        skipLibCheck: true,
        noEmit: true,
        jsx: 'preserve',
        paths: { jinge: [path.join(runtimeRoot, 'types/index.d.ts')] },
      },
      files: [generated],
    }),
  );
  const types = spawnSync(path.join(root, 'node_modules/.bin/tsc'), ['-p', config], {
    encoding: 'utf8',
  });
  assert.equal(types.status, 0, types.stdout + types.stderr);
});

test('S03 captured mutable bindings and shadowed callback parameters retain their identity', async () => {
  const source = `import {t,vm} from 'jinge';export const state=vm({count:0});export function A(){let count=0;return <>{t('<B>child</B>',{B:child=><button on:click={()=>{count++;state.count=count}}>{child}:{((child)=>child)('shadow')}:{state.count}</button>},{key:'mutable'})}</>}`;
  await init(dictionary(source, [['mutable', '<B>child</B>']]), 'mutable');
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'child:shadow:0');
    document.querySelector('button').click();
    await flush();
    document.querySelector('button').click();
    await flush();
    assert.equal(view.text(), 'child:shadow:2');
  } finally {
    view.dispose();
  }
});

test('S03 tag and scalar names are checked and message attributes are rejected', () => {
  const source = `import {t} from 'jinge';export const msg=t('<B>child</B>',{B:child=><b>{child}</b>},{key:'invalid'});`;
  for (const text of ['<Unknown/>', '<B title="x">child</B>', '<B>{B}</B>']) {
    assert.throws(
      () => dictionary(source, [['invalid', text]]),
      /rich component not found|do not support attributes|conflict/,
    );
  }
});

test('S03 missing renderers fail explicitly when a dictionary is used with incompatible parameters', async () => {
  const source = `import {t} from 'jinge';export function A(){return <>{t('<B>child</B>',{B:child=><b>{child}</b>},{key:'missing'})}</>}`;
  await init(dictionary(source, [['missing', '<B>child</B>']]), 'missing');
  await assert.rejects(
    () => mount(source.replace('B:child', 'C:child'), { intl: 1 }),
    /Missing rich text tag renderer/,
  );
});

const mappedContent = `<>{row.children.map((child,innerIndex)=><button key={child.key} on:click={()=>{state.last=row.id+':'+child.id+':'+outerIndex+':'+innerIndex}}>{row.id}:{child.id}:{outerIndex}:{innerIndex}</button>)}</>`;
for (const [kind, renderer] of [
  ['arrow', `()=>${mappedContent}`],
  ['function', `function(){return ${mappedContent}}`],
  ['JSX', mappedContent],
])
  test(`S03 rich ${kind} renderer keeps outer map bindings in nested maps and events`, async () => {
    const source = `import {t,vm} from 'jinge';export const state=vm({last:'',rows:[{key:'row',id:'outer',children:[{key:'child',id:'inner'}]}]});export function A(){return <>{state.rows.map((row,outerIndex)=><div key={row.key}>{t('<Body/>',{Body:${renderer}},{key:'rich-map'})}</div>)}</>}`;
    const locale = `rich-map-${kind}`;
    await init(dictionary(source, [['rich-map', '<Body/>']]), locale);
    const view = await mount(source, { intl: 1 });
    try {
      assert.equal(view.text(), 'outer:inner:0:0');
      document.querySelector('button').click();
      assert.equal(view.m.state.last, 'outer:inner:0:0');
      view.m.state.rows[0].id = 'changed';
      await flush();
      assert.equal(view.text(), 'changed:inner:0:0');
      document.querySelector('button').click();
      assert.equal(view.m.state.last, 'changed:inner:0:0');
      await rt.intlSetLocale(`${locale}-zh`);
      await flush();
      assert.equal(view.text(), 'changed:inner:0:0');
      view.m.state.last = '';
      document.querySelector('button').click();
      assert.equal(view.m.state.last, 'changed:inner:0:0');
    } finally {
      view.dispose();
    }
  });

test('S03 nested rich renderers inherit map depth across multiple surrounding loops', async () => {
  const source = `import {t,vm} from 'jinge';const state=vm({groups:[{id:'group',rows:[{id:'row',children:[{id:'child',leaves:[{id:'leaf'}]}]}]}]});export function A(){return <>{state.groups.map(group=><div key={group.id}>{group.rows.map(row=><div key={row.id}>{t('<Body/>',{Body:()=> <>{row.children.map(child=><div key={child.id}>{t('<Inner/>',{Inner:()=> <>{child.leaves.map(leaf=><span key={leaf.id}>{group.id}:{row.id}:{child.id}:{leaf.id}</span>)}</>},{key:'inner-map'})}</div>)}</>},{key:'outer-map'})}</div>)}</div>)}</>}`;
  await init(
    dictionary(source, [
      ['outer-map', '<Body/>'],
      ['inner-map', '<Inner/>'],
    ]),
    'nested-rich-maps',
  );
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'group:row:child:leaf');
  } finally {
    view.dispose();
  }
});

test('S03 nested rich callbacks keep outer and inner children distinct', async () => {
  const source = `import {t} from 'jinge';export function A(){return <>{t('Outer <B>outer</B>',{B:outer=><>{t('Inner <I>inner</I>',{I:inner=><b>{outer}:{inner}</b>},{key:'inner'})}</>},{key:'outer'})}</>}`;
  await init(
    dictionary(source, [
      ['outer', 'Outer <B>outer</B>'],
      ['inner', 'Inner <I>inner</I>'],
    ]),
    'nested-capture',
  );
  const view = await mount(source, { intl: 1 });
  try {
    assert.equal(view.text(), 'Outer Inner outer:inner');
  } finally {
    view.dispose();
  }
});

for (const [translation, initial, updated] of [
  ['你好 {name}', '你好 Ada', '你好 Bob'],
  ['纯文本', '纯文本', '纯文本'],
])
  test(`S03 a translation may remove all rich tags: ${translation}`, async () => {
    const source = `import {t,vm} from 'jinge';export const state=vm({name:'Ada'});export function A(){return <>{t('Hello <B>{name}</B>',{name:state.name,B:child=><b>{child}</b>},{key:'optional-tags'})}</>}`;
    await init(
      dictionary(source, [['optional-tags', 'Hello <B>{name}</B>', translation]]),
      'optional-tags',
    );
    const view = await mount(source, { intl: 1 });
    try {
      assert.equal(view.text(), 'Hello Ada');
      await rt.intlSetLocale('optional-tags-zh');
      await flush();
      assert.equal(view.text(), initial);
      assert.equal(document.querySelectorAll('b').length, 0);
      view.m.state.name = 'Bob';
      await flush();
      assert.equal(view.text(), updated);
      await rt.intlSetLocale('optional-tags-en');
      await flush();
      assert.equal(view.text(), 'Hello Bob');
      assert.equal(document.querySelector('b').textContent, 'Bob');
    } finally {
      view.dispose();
    }
  });

for (const [name, callback] of [
  ['default', `(child='fallback')=><b>{child}</b>`],
  ['destructuring', `({name})=><b>{name}</b>`],
  ['rest', `(...args)=><b/>`],
  ['extra arguments', `(child,other)=><b/>`],
  ['async', `async child=><b>{child}</b>`],
  ['generator', `function* (child){return <b>{child}</b>}`],
  ['own arguments', `function(child){return <b>{arguments[0]}</b>}`],
  ['parameter writes', `child=>{child='changed';return <b>{child}</b>}`],
])
  test(`S03 rejects unsupported callback signature: ${name}`, () => {
    assert.throws(
      () =>
        compile(
          `import {t} from 'jinge';export function A(){return <>{t('<B>child</B>',{B:${callback}},{key:'bad'})}</>}`,
          { intl: 1 },
        ),
      (error) => /国际化富文本回调/.test(error.message) && /fixture.tsx:/.test(error.message),
    );
  });

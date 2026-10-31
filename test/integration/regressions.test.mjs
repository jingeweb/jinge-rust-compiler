import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { behavior, compile, flush, load, mount, root, rt } from './harness.mjs';
const vm = `import {vm,t} from 'jinge';`;

test('C01 Unicode multiline text preserves characters', async () => {
  await behavior('export function A(){return <div>\n你好 😀\n世界\n</div>}', '你好 😀 世界');
});
test('C04 optional call executes and short circuits', async () => {
  await behavior(
    vm +
      `export const state=vm({fn:()=> 'ok'});export function A(){return <div>{state.fn?.()}</div>}`,
    'ok',
    (m) => {
      m.state.fn = undefined;
    },
    'undefined',
  );
});
test('C05 generated parameters do not capture a0', async () => {
  await behavior(
    vm +
      `const a0=10;export const state=vm({n:1});export function A(){return <div>{a0+state.n}</div>}`,
    '11',
    (m) => (m.state.n = 2),
    '12',
  );
});
test('C06 intl replacement identifies individual calls', async () => {
  const v = await mount(
    vm +
      `const format=x=>'['+x+']';export function A(){return <div title={format('x')+t('hello')}/>}`,
    { intl: 1 },
  );
  try {
    assert.equal(document.body.firstElementChild.title, '[x]hello');
  } finally {
    v.dispose();
  }
});
test('C07 SVG nesting and foreignObject namespace', async () => {
  const v = await mount(
    `export function A(){return <svg><g><path/></g><foreignObject><div><svg><circle/></svg></div></foreignObject></svg>}`,
  );
  try {
    for (const tag of ['svg', 'g', 'path', 'foreignObject', 'circle'])
      assert.equal(document.querySelector(tag).namespaceURI, 'http://www.w3.org/2000/svg', tag);
    assert.equal(document.querySelector('div').namespaceURI, 'http://www.w3.org/1999/xhtml');
  } finally {
    v.dispose();
  }
});
test('C08 reactive component attribute supports hyphens', async () => {
  await behavior(
    vm +
      `export const state=vm({n:'hi'});function Child(props){return <p>{props['data-id']}</p>}export function A(){return <Child data-id={state.n}/>}`,
    'hi',
    (m) => (m.state.n = 'bye'),
    'bye',
  );
});
test('C09 constant IDL properties match reactive properties', async () => {
  const v = await mount(
    `export function A(){return <><input checked={false} disabled={false} readOnly={true} value="hello"/><select><option selected={false}>a</option><option selected={true}>b</option></select><div draggable={false} aria-hidden={false}/></>}`,
  );
  try {
    const input = document.querySelector('input');
    assert.equal(input.checked, false);
    assert.equal(input.disabled, false);
    assert.equal(input.readOnly, true);
    assert.equal(input.value, 'hello');
    assert.equal(document.querySelector('select').selectedIndex, 1);
    assert.equal(document.querySelector('div').draggable, false);
    assert.equal(document.querySelector('div').getAttribute('aria-hidden'), 'false');
  } finally {
    v.dispose();
  }
});
for (const [name, jsx] of Object.entries({
  dom: '<div {...attrs}/>',
  props: '<Child {...attrs} a={1}/>',
  children: '<Child {...attrs}><p>hello</p></Child>',
  slot: '<Child {...attrs} slot:a={<b/>}/>',
}))
  test(`C10 rejects spread ${name}`, () =>
    assert.throws(() =>
      compile(`const attrs={};function Child(){return <p/>}export function A(){return ${jsx}}`),
    ));
test('C10 preserves props identity and ref with component spread', async () => {
  await behavior(
    vm +
      `import {ref,expose} from 'jinge';export const child=ref();export const state=vm({n:1});let identical=false;function Child(props){identical=props===state;expose({read:()=>props.n});return <p>{props.n}</p>}export function A(){return <Child {...state} ref={child}/>}export {identical};`,
    '1',
    (m) => {
      assert.equal(m.identical, true);
      assert.equal(m.child.value.read(), 1);
      m.state.n = 2;
    },
    '2',
  );
});
test('C11 object slot injects runtime imports', async () =>
  behavior(`export const opts={'slot:content':<p>hi</p>}`, 'hi', null, null, { slot: true }));
test('C12 map replacement respects local shadowing and shorthand', async () => {
  await behavior(
    vm +
      `export const state=vm({items:[{name:'outer'}]});export function A(){return <div>{state.items.map(item=>{const f=()=>{const item={name:'inner'};return item.name};return <p key={item}>{f()}</p>})}</div>}`,
    'inner',
  );
});
test('C13 const computed key watches state', async () =>
  behavior(
    vm +
      `const key='name';export const state=vm({name:'Alice'});export function A(){return <div>{state[key]}</div>}`,
    'Alice',
    (m) => (m.state.name = 'Bob'),
    'Bob',
  ));
test('C13 mutable computed key still watches selected state field', async () =>
  behavior(
    vm +
      `let key='name';export const state=vm({name:'Alice'});export function A(){return <div>{state[key]}</div>}`,
    'Alice',
    (m) => (m.state.name = 'Bob'),
    'Bob',
  ));
for (const block of [false, true])
  test(`C14 mixed path key, block=${block}, preserves DOM identity`, async () => {
    const v = await mount(
      vm +
        `export const state=vm({items:[{meta:{id:1},name:'a'},{meta:{id:2},name:'b'}]});export function A(){return <ul>{state.items.map(item=>${block ? '{return ' : ''}<li key={item.meta['id']}>{item.name}</li>${block ? '}' : ''})}</ul>}`,
    );
    try {
      const a = document.querySelector('li');
      v.m.state.items.reverse();
      await flush();
      assert.equal(v.text(), 'ba');
      assert.equal(document.querySelectorAll('li')[1], a);
    } finally {
      v.dispose();
    }
  });
test('C15 .ts parser accepts generics and angle assertions', async () => {
  const { module: m } = await load(
    `export const identity=<T>(x:T):T=>x;export const n=<number>123;`,
    { type: 1 },
  );
  assert.equal(m.identity(m.n), 123);
});
test('C16 .ts intl transforms without component conversion', () => {
  const r = compile(`import {t} from 'jinge';export const label=t('hello');`, { type: 1, intl: 2 });
  assert.match(r.code, /m3HSJL/);
  assert.equal(r.parsedComponents, '');
});
test('C17 conditional slot inherits custom host', async () =>
  behavior(
    `export function A(props,hostCustom){return <>{props.children?<p>yes</p>:<p>no</p>}</>}`,
    'no',
  ));
test('C18 intl attribute params update and locale changes', async () => {
  const key = 'greeting';
  await rt.intlInit({ defaultLocale: 'en', dicts: { en: { [key]: (p) => `Hello ${p.name}` } } });
  const v = await mount(
    vm +
      `export const state=vm({name:'Alice'});export function A(){return <div title={t('Hello {name}',{name:state.name},{key:'greeting'})}/>}`,
    { intl: 1 },
  );
  try {
    assert.equal(document.body.firstElementChild.title, 'Hello Alice');
    v.m.state.name = 'Bob';
    await flush();
    assert.equal(document.body.firstElementChild.title, 'Hello Bob');
  } finally {
    v.dispose();
  }
});
test('C19 rich text default fallback argument position', async () =>
  behavior(
    vm + `export function A(){return <div>{t('Hello <B>world</B>',{B:text=><b>{text}</b>})}</div>}`,
    'Hello <B>world</B>',
    null,
    null,
    { intl: 1 },
  ));
for (const scenario of [
  'csv',
  'escape',
  'rich-key',
  'rich-binding',
  'key-trim',
  'cli',
  'extract',
  'helper-cli',
  'empty',
])
  test(`intl tools: ${scenario}`, () => {
    const r = spawnSync(process.execPath, [`${root}/test/.compiled/tools.js`, scenario], {
      encoding: 'utf8',
    });
    assert.equal(r.status, 0, r.stdout + r.stderr);
  });
for (const [name, body] of Object.entries({
  early: `if(props.ok)return <p>yes</p>;return <p>no</p>`,
  destructure: `return state.items.map(({name})=><p>{name}</p>)`,
  default: `return state.items.map((item={})=><p>{item.name}</p>)`,
  rest: `return state.items.map((...items)=><p>{items[0]}</p>)`,
  third: `return state.items.map((item,index,array)=><p>{array.length}</p>)`,
}))
  test(`diagnostics reject ${name}`, () =>
    assert.throws(() =>
      compile(vm + `const state=vm({items:[]});export function A(props){${body}}`),
    ));
test('trailing empty statements are accepted', async () =>
  behavior(`export function A(){return <p>yes</p>;;}`, 'yes'));
test('syntax and unsupported slot errors are ordinary errors; next compile succeeds', () => {
  for (const source of [
    `export function A(){return <div>}`,
    `const value={};export const opts={'slot:content':value?.['slot:a']}`,
  ])
    assert.throws(
      () => compile(source),
      (e) => !e.message.includes('internal error'),
    );
  assert.match(compile('export function A(){return <p/>}').code, /createEle/);
});
test('binding respects sourcemap flag', () => {
  assert.equal(compile('export const n=1', { type: 1 }).map, '');
  assert.ok(JSON.parse(compile('export const n=1', { type: 1, map: true }).map).mappings);
});
// Warnings are captured from a separate process because SWC writes directly to stderr.
for (const [name, decl, expression, warn] of [
  ['local-object', 'const state={name:1}', 'state.name', true],
  ['local-array', 'const items=[]', 'items.map(item=><p key={item}>{item}</p>)', true],
  ['vm', 'const state=vm({name:1})', 'state.name', false],
  ['alias', 'const state=vm({name:1});const alias=state', 'alias.name', false],
  ['unknown', 'const state=(()=>({name:1}))()', 'state.name', false],
  ['mutable-key', 'const state=vm({name:1});let key="name"', 'state[key]', true],
  ['const-key', 'const state=vm({name:1});const key="name"', 'state[key]', false],
])
  test(`scope diagnostics: ${name}`, () => {
    const source = vm + `${decl};export function A(){return <div>{${expression}}</div>}`;
    const r = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
      encoding: 'utf8',
    });
    assert.equal(r.status, 0, r.stderr);
    assert.equal(/warning:/.test(r.stderr), warn, r.stderr);
  });

test('C14 callback-local key must not escape its scope', () => {
  assert.throws(
    () =>
      compile(
        vm +
          `const state=vm({items:[{id:1}]});export function A(){return <>{state.items.map(item=>{const id=item.id;return <p key={id}/>})}</>}`,
      ),
    /key/,
  );
});
test('C14 quoted and backslash property keys remain valid', async () => {
  const key = "quote'\\line\n";
  await behavior(
    vm +
      `const items=vm([{${JSON.stringify(key)}:1}]);export function A(){return <>{items.map(item=><p key={item[${JSON.stringify(key)}]}>ok</p>)}</>}`,
    'ok',
  );
});
test('C18 reactive intl params evaluate user functions only once initially', async () => {
  await rt.intlInit({ defaultLocale: 'once', dicts: { once: { once: (p) => `Hello ${p.name}` } } });
  const v = await mount(
    vm +
      `export let count=0;const format=x=>{count++;return x};export const state=vm({name:'a'});export function A(){return <div title={t('Hello {name}',{name:format(state.name)},{key:'once'})}/>}`,
    { intl: 1 },
  );
  try {
    assert.equal(v.m.count, 1);
    assert.equal(document.querySelector('div').title, 'Hello a');
  } finally {
    v.dispose();
  }
});
test('C12 nested block, destructuring, catch bindings and shorthand retain identity', async () => {
  await behavior(
    vm +
      `const state=vm({items:[{name:'outer'}]});export function A(){return <>{state.items.map(item=>{const f=()=>{let out='';{const {item}={item:'destructured'};out+=item}try{throw 'caught'}catch(item){out+=item}return out};const copy={item};return <p key={item}>{f()}:{copy.item.name}</p>})}</>}`,
    'destructuredcaught:outer',
  );
});

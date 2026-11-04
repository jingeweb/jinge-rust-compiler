import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { compile, load, root } from './harness.mjs';

for (const expression of [
  'props.obj?.[read()+props.n]',
  'props.obj?.[props.key]',
  'props.obj?.[key]',
  'props.obj?.child[props.key]',
  'props.obj?.["child"][props.key]',
  'props.obj?.[0]?.[props.key]',
  'props.obj[props.key]?.[read()+props.n]',
  'props.obj?.[props.key]()',
  'props.obj?.child[props.key]?.()',
])
  test(`optional computed keys report unsupported short-circuit extraction: ${expression}`, () => {
    for (const template of [
      `<p>{${expression}}</p>`,
      `<p title={${expression}}/>`,
      `<Child value={${expression}}/>`,
      `<>{t('Value {value}',{value:${expression}})}</>`,
      `<>{props.children({value:${expression}})}</>`,
    ]) {
      const source = `import {t} from 'jinge';const key='x';function read(){return 'x'}function Child(props){return <p>{props.value}</p>}export function A(props){return ${template}}`;
      assert.throws(
        () => compile(source, { intl: 1 }),
        (error) =>
          /可选链.*动态计算属性/.test(error.message) &&
          /fixture\.tsx:1:/.test(error.message) &&
          !error.message.includes('internal error'),
        template,
      );
    }
  });

for (const expression of [
  'props.obj?.["name"]',
  'props.obj?.[0]',
  'props.obj?.child',
  'props.obj[props.key]?.name',
  'props.obj[props.key]?.()',
  'props.obj[read()+props.n]',
  'props.obj?.["label"]()',
  'props.table[props.key?.id]',
])
  test(`optional computed diagnostic preserves supported paths: ${expression}`, () => {
    assert.doesNotThrow(() =>
      compile(`function read(){return 'x'}export function A(props){return <p>{${expression}}</p>}`),
    );
  });

test('optional computed diagnostic skips deferred functions and plain TS expressions', () => {
  assert.doesNotThrow(() =>
    compile(
      `function read(){return 'x'}function Child(props){return <p/>}export function A(props){return <Child config={{active:props.active,read:()=>props.obj?.[read()+props.n]}}/>}`,
    ),
  );
  assert.doesNotThrow(() =>
    compile(
      `function read(){return 'x'}export function A(props){return <button on:click={()=>{props.obj?.[read()+props.n]}}>Click</button>}`,
    ),
  );
  assert.doesNotThrow(() =>
    compile(
      `function read(){return 'x'}export function value(props){return props.obj?.[read()+props.n]}`,
      { type: 1 },
    ),
  );
});

for (const [name, source, pattern] of [
  ['component props destructuring', 'export function A({name}){return <p>{name}</p>}', /参数/],
  ['invalid host', 'export function A(props,other){return <p/>}', /host/],
  ['duplicate ref', 'export function A(){return <p ref={a} ref={b}/>}', /ref/],
  ['literal ref', 'export function A(){return <p ref="x"/>}', /ref/],
  ['DOM named slot', 'export function A(){return <div slot:x="text"/>}', /slot/],
  [
    'missing named slot value',
    'function Child(){return <p/>}export function A(){return <Child slot:x/>}',
    /属性值/,
  ],
  ['unknown namespace', 'export function A(){return <p unknown:x={1}/>}', /命名空间/],
  ['children prop', 'export function A(){return <p children="x"/>}', /children/],
  [
    'JSX prop',
    'function Child(){return <p/>}export function A(){return <Child content={<b/>}/>}',
    /slot/,
  ],
  ['event literal', 'export function A(){return <p on:click="bad"/>}', /事件/],
  ['event boolean', 'export function A(){return <p on:click/>}', /事件/],
  ['slot invalid argument', 'export function A(props){return <>{props.children(1)}</>}', /Slot/],
  [
    'slot multiple arguments',
    'export function A(props){return <>{props.children({}, {})}</>}',
    /参数/,
  ],
  ['slot bare vm', 'export function A(props){return <>{props.children(state)}</>}', /ViewModel/],
  [
    'slot duplicate spread',
    'export function A(props){return <>{props.children({...a,...b})}</>}',
    /透传/,
  ],
  ['member call result', 'export function A(){return <p>{factory().name}</p>}', /Call/],
  [
    'intl invalid message',
    "import {t} from 'jinge';export function A(){return <p>{t(message)}</p>}",
    /字符串/,
  ],
  [
    'intl spread params',
    "import {t} from 'jinge';export function A(){return <p>{t('hi',{...params})}</p>}",
    /解构/,
  ],
  [
    'spread expression',
    'function Child(){return <p/>}export function A(){return <Child {...getProps()}/>}',
    /Ident/,
  ],
  [
    'multiple spread',
    'function Child(){return <p/>}export function A(){return <Child {...a} {...b}/>}',
    /透传/,
  ],
])
  test(`diagnostic: ${name}`, () => {
    assert.throws(
      () => compile(source, { intl: 1 }),
      (e) =>
        pattern.test(e.message) &&
        e.message.includes('fixture.tsx:') &&
        !e.message.includes('internal error'),
    );
    assert.ok(compile('export const n=1', { type: 1 }).code);
  });
test('anonymous default component warns but compiles; nested component HMR stays in module scope', async () => {
  const r = await load('export default function(){return <p>ok</p>}');
  assert.equal(typeof r.module.default, 'function');
  assert.equal(r.result.parsedComponents, '');
  const nested = compile(
    'export function factory(){function Inner(){return <p>ok</p>}return Inner}',
  );
  assert.equal(nested.parsedComponents, '');
});
for (const [name, source, warn] of [
  ['imported', "import {state} from './state';export function A(){return <p>{state.n}</p>}", false],
  ['factory', 'const state=createState();export function A(){return <p>{state.n}</p>}', false],
  [
    'context',
    "import {context} from 'jinge';const state=context('state');export function A(){return <p>{state.n}</p>}",
    false,
  ],
  [
    'local component variable',
    'export function A(){const state={n:1};return <p>{state.n}</p>}',
    true,
  ],
  [
    'function callback is not template evaluation',
    'export function A(){const state={n:1};return <button on:click={()=>state.n++}/>}',
    false,
  ],
  [
    'vm alias import',
    "import {vm as makeVm} from 'jinge';const state=makeVm({n:1});export function A(){return <p>{state.n}</p>}",
    false,
  ],
  [
    'shadowed vm import',
    "import {vm} from 'jinge';function make(vm){const state=vm({n:1});return state}export function A(){const state=make(x=>x);return <p>{state.n}</p>}",
    false,
  ],
  [
    'shadowed binding',
    "import {vm} from 'jinge';const state=vm({n:1});export function A(){const state={n:2};return <p>{state.n}</p>}",
    true,
  ],
])
  test(`warning boundary: ${name}`, () => {
    const r = spawnSync(process.execPath, [pathToWarnings(), source], { encoding: 'utf8' });
    assert.equal(r.status, 0, r.stderr);
    assert.equal(/warning:/.test(r.stderr), warn, r.stderr);
  });
function pathToWarnings() {
  return `${root}/test/integration/warnings.mjs`;
}
for (const kind of ['const', 'let'])
  test(`computed destructured key declaration: ${kind}`, () => {
    const source = `import {vm} from 'jinge';const state=vm({name:'a'});${kind} {key}={key:'name'};export function A(){return <p>{state[key]}</p>}`;
    const r = spawnSync(process.execPath, [pathToWarnings(), source], { encoding: 'utf8' });
    assert.equal(r.status, 0, r.stderr);
    assert.equal(/warning:/.test(r.stderr), kind === 'let', r.stderr);
  });

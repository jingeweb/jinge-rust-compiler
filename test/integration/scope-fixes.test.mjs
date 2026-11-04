import assert from 'node:assert/strict';
import { test } from 'node:test';

import { compile, flush, mount } from './harness.mjs';
for (const expression of [
  'item=>item.id===1',
  'function(item){return item.id===1}',
  '()=>{const item={id:1};return item.id===1}',
  '(item={id:1})=>item.id===1',
])
  test(`S01 callback keeps its scope: ${expression}`, async () => {
    const v = await mount(
      `import {vm} from 'jinge';function Child(props){return <span>{props.config.test({id:1})?'matched':'miss'}</span>}export function A(){const state=vm({active:true});return <Child config={{active:state.active,test:${expression}}}/>}`,
    );
    try {
      assert.equal(v.text(), 'matched');
    } finally {
      v.dispose();
    }
  });
test('S01 some callback preserves parameter members', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({rows:[{id:1}],id:1});export function A(){return <span>{state.rows.some(row=>row.id===state.id)}:{state.id}</span>}`,
  );
  try {
    assert.equal(v.text(), 'true:1');
    v.m.state.rows = [{ id: 2 }];
    await flush();
    assert.equal(v.text(), 'false:1');
  } finally {
    v.dispose();
  }
});
test('S01 captured callback keeps live reads and receiver', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({active:true,count:0});export let saved;function Child(props){saved=props.config.read;return <span>{props.config.test()}</span>}export function A(){return <Child config={{active:state.active,read:()=>state.count,test(){return this.active}}}/>}`,
  );
  try {
    assert.equal(v.text(), 'true');
    v.m.state.count = 1;
    await flush();
    assert.equal(v.m.saved(), 1);
  } finally {
    v.dispose();
  }
});
for (const write of [
  'item={id:2}',
  '({item}={item:{id:2}})',
  'index++',
  '[item]=[{id:2}]',
  'for(item of [{id:2}]){}',
]) {
  test(`S04 rejects rebinding map parameters: ${write}`, () => {
    assert.throws(
      () =>
        compile(
          `import {vm} from 'jinge';export function A(){const state=vm({rows:[{id:1}]});return <>{state.rows.map((item,index)=><button key={item.id} on:click={()=>{${write}}}>{index}</button>)}</>}`,
        ),
      /map.*参数.*重新赋值/,
    );
  });
}
test('S04 accepts member writes and shadowed inner parameters', () => {
  assert.doesNotThrow(() =>
    compile(
      `export function A(props){return <>{props.rows.map((item,index)=><button key={item.id} on:click={()=>{item.id=2;((item)=>{item={id:3}})({id:0})}}>{index}</button>)}</>}`,
    ),
  );
});
test('S07 preserves a shadowed undefined', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({on:true});export function A(){const undefined='business';return <span>{state.on?undefined:<b>fallback</b>}</span>}`,
  );
  try {
    assert.equal(v.text(), 'business');
    v.m.state.on = false;
    await flush();
    assert.equal(v.text(), 'fallback');
  } finally {
    v.dispose();
  }
});

test('S07 keeps global undefined empty and distinguishes nested arrow bindings', async () => {
  const v = await mount(
    `function Child(input){return <>{input.children({name:'business'})}</>}export function A(){return <><Child>{slot=>{const undefined=slot.name;return <span>{true?undefined:<b/>}</span>}}</Child><i>{false?<b>hidden</b>:undefined}</i></>}`,
  );
  try {
    assert.equal(v.text(), 'business');
  } finally {
    v.dispose();
  }
});
for (const [name, expression, expected] of [
  ['arguments', 'arguments[0].name', 'provided'],
  ['self reference', 'render.name', 'render'],
])
  test(`S05 normal function slot retains ${name}`, async () => {
    const v = await mount(
      `function Child(input){return <>{input.children({name:'provided'})}</>}export function A(){return <Child>{function render(slot){return <span>{${expression}}</span>}}</Child>}`,
    );
    try {
      assert.equal(v.text(), expected);
    } finally {
      v.dispose();
    }
  });

test('S06 recognizes a default re-export forwarding component', () => {
  assert.ok(
    compile('const forward=input=>input.children;export {forward as default}').code.includes(
      'renderSlotFunction$jg$(',
    ),
  );
});
for (const decl of [
  'let Alias=read;Alias=Component;',
  'let Alias=read;Alias=Component;const Final=Alias;',
])
  test(`S06 rejects writable component aliases: ${decl}`, () => {
    assert.throws(
      () =>
        compile(
          `function read(node){return node.children}function Component(input){return input.children}${decl}export function A(){return <${decl.includes('Final') ? 'Final' : 'Alias'}>child</${decl.includes('Final') ? 'Final' : 'Alias'}>}`,
        ),
      /组件引用.*重新赋值/,
    );
  });

test('C03 callback-local expression dependencies (promoted from pending)', async () => {
  const v = await mount(
    `import {vm} from 'jinge';const state=vm({items:[{active:true}]});export function A(){return <p>{state.items.some(item=>item.active)}</p>}`,
  );
  try {
    assert.equal(v.text(), 'true');
  } finally {
    v.dispose();
  }
});

for (const body of ['var item={id:2};', 'for(var item of [{id:2}]){}'])
  test(`S04 rejects parameter reinitialization: ${body}`, () => {
    assert.throws(
      () =>
        compile(
          `export function A(props){return <>{props.rows.map(item=>{${body}return <b key={item.id}>{item.id}</b>})}</>}`,
        ),
      /map.*参数.*重新赋值/,
    );
  });

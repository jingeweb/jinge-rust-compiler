import assert from 'node:assert/strict';
import { test } from 'node:test';

import { flush, load, mount, rt } from './harness.mjs';
const vm = `import {vm} from 'jinge';`;
test('keyed list mutations preserve order, row identity, values, and destroy watchers', async () => {
  const v = await mount(
    vm +
      `export const state=vm({items:[{id:1,name:'a'},{id:2,name:'b'},{id:3,name:'c'}]});export function A(){return <ul>{state.items.map((item,index)=><li key={item.id}>{index}:{item.name};</li>)}</ul>}`,
  );
  try {
    const list = v.m.state.items;
    const nodes = new Map();
    const check = async () => {
      await flush();
      const data = v.m.state.items;
      assert.equal(v.text(), data.map((x, i) => `${i}:${x.name};`).join(''));
      for (const [i, item] of data.entries()) {
        const node = document.querySelectorAll('li')[i];
        if (nodes.has(item.id)) assert.equal(node, nodes.get(item.id));
        nodes.set(item.id, node);
      }
    };
    await check();
    list.reverse();
    await check();
    list.push({ id: 4, name: 'd' });
    await check();
    list.unshift({ id: 5, name: 'e' });
    await check();
    list.splice(1, 1, { id: 6, name: 'f' });
    await check();
    list[0].name = 'E';
    await check();
    list.sort((a, b) => a.id - b.id);
    await check();
    list.pop();
    await check();
    list.shift();
    await check();
    list.length = 1;
    await check();
    v.m.state.items = [];
    await check();
  } finally {
    v.dispose();
  }
});
test('direct vm array and unkeyed list handle structural mutations', async () => {
  const v = await mount(
    vm +
      `export const items=vm(['a','b','c']);export function A(){return <ul>{items.map(item=><li>{item}</li>)}</ul>}`,
  );
  try {
    const first = document.querySelector('li');
    v.m.items.reverse();
    await flush();
    assert.equal(v.text(), 'cba');
    assert.equal(document.querySelector('li'), first);
    v.m.items.fill('x', 1);
    await flush();
    assert.equal(v.text(), 'cxx');
    v.m.items.copyWithin(0, 1);
    await flush();
    assert.equal(v.text(), 'xxx');
    v.m.items.splice(0);
    await flush();
    assert.equal(v.text(), '');
  } finally {
    v.dispose();
  }
});
test('dynamic paths rebind, and unmount cancels pending updates', async () => {
  const v = await mount(
    vm +
      `export const state=vm({key:'a',values:{a:'A',b:'B'}});export function A(){return <p>{state.values[state.key]+'!'}</p>}`,
  );
  const p = document.querySelector('p');
  try {
    v.m.state.key = 'b';
    await flush();
    assert.equal(p.textContent, 'B!');
    v.m.state.values.a = 'old';
    await flush();
    assert.equal(p.textContent, 'B!');
    v.m.state.values.b = 'new';
    await flush();
    assert.equal(p.textContent, 'new!');
    v.m.state.values.b = 'pending';
  } finally {
    v.dispose();
  }
  await flush();
  assert.equal(p.textContent, 'new!');
  v.m.state.values.b = 'destroyed';
  await flush();
  assert.equal(p.textContent, 'new!');
});
test('dynamic handlers detach old listeners and unmount clears bindings', async () => {
  const v = await mount(
    vm +
      `const first=()=>state.n++;const second=()=>state.n+=10;export const state=vm({n:0,handler:first});export {second};export function A(){return <button on:click={state.handler}>{state.n}</button>}`,
  );
  const button = document.querySelector('button');
  try {
    button.click();
    await flush();
    assert.equal(v.text(), '1');
    v.m.state.handler = v.m.second;
    await flush();
    button.click();
    await flush();
    assert.equal(v.text(), '11');
    v.m.state.handler = null;
    await flush();
    button.click();
    await flush();
    assert.equal(v.text(), '11');
    v.m.state.handler = v.m.second;
    await flush();
  } finally {
    v.dispose();
  }
  button.click();
  assert.equal(v.m.state.n, 11);
});
test('capture and doubleClick event mappings execute in DOM order', async () => {
  const v = await mount(
    `export const log=[];export function A(){return <div on:clickCapture={()=>log.push('capture')} on:click={()=>log.push('bubble')}><button on:click={()=>log.push('target')} on:doubleClick={()=>log.push('double')}>go</button></div>}`,
  );
  try {
    const b = document.querySelector('button');
    b.click();
    b.dispatchEvent(new window.MouseEvent('dblclick', { bubbles: true }));
    assert.deepEqual(v.m.log, ['capture', 'target', 'bubble', 'double']);
  } finally {
    v.dispose();
  }
});
test('refs, expose, lifecycle, conditional destruction, and context inheritance', async () => {
  const v = await mount(
    `import {vm,ref,expose,onMount,onUnmount,context} from 'jinge';export const state=vm({show:true,name:'a'});export const child=ref();export const events=[];function Child(props){const shared=context('test');expose({hello:()=>props.name});onMount(()=>{events.push('mount');return ()=>events.push('cleanup')});onUnmount(()=>events.push('unmount'));return <p>{shared.name}:{props.name}</p>}export function A(){context('test',state);return <>{state.show&&<Child ref={child} name={state.name}/>}</>}`,
  );
  try {
    assert.equal(v.text(), 'a:a');
    assert.equal(v.m.child.value.hello(), 'a');
    assert.deepEqual(v.m.events, ['mount']);
    v.m.state.name = 'b';
    await flush();
    assert.equal(v.text(), 'b:b');
    assert.equal(v.m.child.value.hello(), 'b');
    v.m.state.show = false;
    await flush();
    assert.equal(v.text(), '');
    assert.ok(v.m.events.includes('cleanup'));
    assert.ok(v.m.events.includes('unmount'));
    v.m.state.show = true;
    await flush();
    assert.equal(v.text(), 'b:b');
  } finally {
    v.dispose();
  }
  assert.equal(v.m.events.filter((e) => e === 'unmount').length, 2);
});
test('default/named scoped slots and object slot forwarding react to vm params', async () => {
  const v = await mount(
    vm +
      `export const state=vm({n:1});function Child(props){return <section>{props['slot:head']}{props.children({...state})}</section>}export const opts={'slot:head':<b>head</b>};export function A(){return <Child slot:head={opts['slot:head']}>{slot=><p>{slot.n}</p>}</Child>}`,
  );
  try {
    assert.equal(v.text(), 'head1');
    v.m.state.n = 2;
    await flush();
    assert.equal(v.text(), 'head2');
  } finally {
    v.dispose();
  }
});
test('Dynamic switches component type with reactive props', async () => {
  const v = await mount(
    `import {vm,Dynamic} from 'jinge';function One(props){return <b>{props.label}</b>}function Two(props){return <i>{props.label}</i>}export const state=vm({fc:One,label:'a'});export {Two};export function A(){return <Dynamic fc={state.fc} label={state.label}/>}`,
  );
  try {
    assert.equal(v.text(), 'a');
    assert.ok(document.querySelector('b'));
    v.m.state.fc = v.m.Two;
    v.m.state.label = 'b';
    await flush();
    assert.equal(v.text(), 'b');
    assert.ok(document.querySelector('i'));
  } finally {
    v.dispose();
  }
});
test('intl text, params, attributes and rich components update on locale changes', async () => {
  const dict = await load(`export function Rich(props){return <strong>{props.name}</strong>}`);
  await rt.intlInit({
    defaultLocale: 'test-en',
    dicts: {
      'test-en': { NhX4DJ: 'Hello', param: (p) => `Hi ${p.name}`, rich: dict.module.Rich },
      'test-zh': { NhX4DJ: '你好', param: (p) => `你好 ${p.name}`, rich: dict.module.Rich },
    },
  });
  const v = await mount(
    `import {vm,t} from 'jinge';export const state=vm({name:'a'});export function A(){return <><span title={t('Hello {name}',{name:state.name},{key:'param'})}>{t('Hello')}</span><p>{t('Hello {name}',{name:state.name},{key:'param'})}</p>{t('Hi <B>{name}</B>',{name:state.name,B:x=><b>{x}</b>},{key:'rich'})}</>}`,
    { intl: 1 },
  );
  try {
    assert.equal(v.text(), 'HelloHi aa');
    v.m.state.name = 'b';
    await flush();
    assert.equal(v.text(), 'HelloHi bb');
    await rt.intlSetLocale('test-zh');
    await flush();
    assert.equal(v.text(), '你好你好 bb');
    assert.equal(document.querySelector('span').title, '你好 b');
  } finally {
    v.dispose();
  }
  await rt.intlSetLocale('test-en');
  await flush();
});

test('Portal renders into its target, updates and removes content on unmount', async () => {
  const v = await mount(
    `import {vm,Portal} from 'jinge';export const state=vm({n:1});export const target=document.createElement('aside');export function A(){return <Portal target={target}><b>{state.n}</b></Portal>}`,
  );
  try {
    assert.equal(v.m.target.textContent, '1');
    v.m.state.n = 2;
    await flush();
    assert.equal(v.m.target.textContent, '2');
  } finally {
    v.dispose();
  }
  assert.equal(v.m.target.textContent, '');
});
test('Lazy loader renders loading and resolved compiled components', async () => {
  const v = await mount(
    `import {Lazy} from 'jinge';export let resolve;const promise=new Promise(r=>resolve=r);function Loaded(props){return <b>{props.name}</b>}export function finish(){resolve(Loaded)}export function A(){return <Lazy loader={()=>promise} name="loaded" slot:loading={<i>loading</i>}/>}`,
  );
  try {
    assert.equal(v.text(), 'loading');
    v.m.finish();
    await flush();
    assert.equal(v.text(), 'loaded');
  } finally {
    v.dispose();
  }
});

test('HMR replaces instances from compiled components and preserves reactive props', async () => {
  rt.initHmr();
  const hmr = window.__JINGE_HMR__;
  const props = rt.vm({ n: 1 });
  const v = await mount(`export function A(props){return <p>old:{props.n}</p>}`, {
    props,
    beforeRender(m) {
      hmr.registerFunctionComponent(m.A, 'test::A');
    },
  });
  try {
    assert.equal(v.text(), 'old:1');
    const next = await load(`export function A(props){return <b>new:{props.n}</b>}`);
    hmr.registerFunctionComponent(next.module.A, 'test::A');
    hmr.replaceComponentInstance(next.module.A);
    assert.equal(v.text(), 'new:1');
    assert.equal(hmr.getLatestFunctionComponent(v.m.A), next.module.A);
    props.n = 2;
    await flush();
    assert.equal(v.text(), 'new:2');
  } finally {
    v.dispose();
    delete window.__JINGE_HMR__;
  }
});

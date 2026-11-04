import assert from 'node:assert/strict';
import { test } from 'node:test';

import { behavior, flush, mount, compile } from './harness.mjs';
test('M01 string member methods follow primitive receiver replacement', async () => {
  await behavior(
    `import {vm} from 'jinge';export const state=vm({text:' Alice '});export function A(){return <p>{state.text.trim()}</p>}`,
    'Alice',
    (m) => {
      m.state.text = ' Bob ';
    },
    'Bob',
  );
});
test('M02 a shared method follows object replacement and method-only replacement', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export function label(){return this.name;}export const state=vm({service:{name:'A',label}});export function A(){return <p>{state.service.label()}</p>}`,
  );
  try {
    assert.equal(v.text(), 'A');
    v.m.state.service = { name: 'B', label: v.m.label };
    await flush();
    assert.equal(v.text(), 'B');
    v.m.state.service.label = function () {
      return this.name + '!';
    };
    await flush();
    assert.equal(v.text(), 'B!');
  } finally {
    v.dispose();
  }
});
test('M03 native methods retain their receiver and explicit argument dependencies remain reactive', async () => {
  await behavior(
    `import {vm} from 'jinge';export const state=vm({text:'hello',needle:'ell'});export function A(){return <p>{state.text.includes(state.needle)}:{state.text.slice(1,3)}</p>}`,
    'true:el',
    (m) => {
      m.state.needle = 'no';
    },
    'false:el',
  );
});
test('M04 dynamic receiver indices and method names are subscribed without losing this', async () => {
  const v = await mount(
    `import {vm} from 'jinge';function label(){return this.name;}export const state=vm({index:0,key:'label',items:[{name:'A',label,other:label},{name:'B',label,other(){return this.name+'!';}}]});export function A(){return <p>{state.items[state.index][state.key]()}</p>}`,
  );
  try {
    assert.equal(v.text(), 'A');
    v.m.state.index = 1;
    v.m.state.key = 'other';
    await flush();
    assert.equal(v.text(), 'B!');
  } finally {
    v.dispose();
  }
});
test('M05 method closure fields are not implicitly deep subscribed', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({service:{name:'A',label(){return this.name;}}});export function A(){return <p>{state.service.label()}</p>}`,
  );
  try {
    v.m.state.service.name = 'B';
    await flush();
    assert.equal(v.text(), 'A');
  } finally {
    v.dispose();
  }
});
test('M06 root receiver and parenthesized member calls retain normal call syntax', async () => {
  await behavior(
    `import {vm} from 'jinge';export const model=vm({name:'A',label(){return this.name;}});export function A(){return <p>{(model.label)()}</p>}`,
    'A',
    (m) => {
      m.model.label = function () {
        return this.name + '!';
      };
    },
    'A!',
  );
});
test('M07 optional receivers/calls short circuit argument invocation but retain receiver binding', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export let calls=0;function arg(){calls++;return '!';}export const state=vm({service:null});export function A(){return <p>{state.service?.label(arg()) ?? 'none'}</p>}`,
  );
  try {
    assert.equal(v.text(), 'none');
    assert.equal(v.m.calls, 0);
    v.m.state.service = {
      name: 'A',
      label(suffix) {
        return this.name + suffix;
      },
    };
    await flush();
    assert.equal(v.text(), 'A!');
    assert.equal(v.m.calls, 1);
  } finally {
    v.dispose();
  }
});
test('M08 optional method lookup skips absent methods and observes later replacement', async () => {
  await behavior(
    `import {vm} from 'jinge';export const state=vm({service:{name:'A',label:undefined}});export function A(){return <p>{state.service.label?.() ?? 'none'}</p>}`,
    'none',
    (m) => {
      m.state.service.label = function () {
        return this.name;
      };
    },
    'A',
  );
});
test('M09 passing a member function as a value must not bind it implicitly', () => {
  const result = compile(
    `export function A(props){return <button on:click={props.handler}>click</button>}`,
  );
  assert.doesNotMatch(result.code, /MethodPathWatcher\$jg\$\(/);
});
test('M10 spread arguments and nested member calls retain dependency order', async () => {
  await behavior(
    `import {vm} from 'jinge';export const state=vm({text:' abc ',start:0,end:2});export function A(){return <p>{state.text.slice(...[state.start,state.end])}:{state.text.includes(state.text.trim())}</p>}`,
    ' a:true',
    (m) => {
      m.state.start = 2;
    },
    ':true',
  );
});

// Promoted from pending C02: receiver semantics are now defined and implemented.
test('C02 member receivers retain this without requiring a body-dependency collector', async () => {
  await behavior(
    `import {vm} from 'jinge';const state=vm({user:{n:7,get(){return this.n}}});export function A(){return <p>{state.user.get()}</p>}`,
    '7',
  );
});

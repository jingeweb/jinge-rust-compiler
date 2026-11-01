import assert from 'node:assert/strict';
import { test } from 'node:test';

import { behavior, mount } from './harness.mjs';
const vm = `import {vm} from 'jinge';`;
test(
  'C02 method receivers and method body dependencies',
  { todo: '已决定本轮不修复；需讨论接收者和依赖规则' },
  async () => {
    await behavior(
      vm +
        `const state=vm({user:{n:7,get(){return this.n}}});export function A(){return <p>{state.user.get()}</p>}`,
      '7',
    );
  },
);
test(
  'C03 callback-local expression dependencies',
  { todo: '已决定本轮不修复；需讨论回调依赖边界' },
  async () => {
    await behavior(
      vm +
        `const state=vm({items:[{active:true}]});export function A(){return <p>{state.items.some(item=>item.active)}</p>}`,
      'true',
    );
  },
);
test(
  'callback-local computed map key extraction',
  { todo: '目前明确拒绝；支持它需要局部计算和外提分析' },
  async () => {
    await behavior(
      vm +
        `const state=vm({items:[{id:1}]});export function A(){return <>{state.items.map(item=>{const id=item.id;return <p key={id}>ok</p>})}</>}`,
      'ok',
    );
  },
);
test(
  'optional computed member preserves side-effect short circuit',
  { todo: '短路依赖提取需统一设计，本轮仅修复普通可选函数调用' },
  async () => {
    const v = await mount(
      vm +
        `export let count=0;const key=()=>{count++;return 'x'};const state=vm({obj:null,n:0});export function A(){return <p>{state.obj?.[key()+state.n]}</p>}`,
    );
    try {
      assert.equal(v.m.count, 0);
    } finally {
      v.dispose();
    }
  },
);

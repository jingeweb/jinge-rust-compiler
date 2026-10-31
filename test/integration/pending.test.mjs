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
  'SVG context across independent component boundaries',
  { todo: '需新增跨组件 namespace 协议，本轮不扩展运行时接口' },
  async () => {
    const v = await mount(
      `function Shape(){return <path/>}export function A(){return <svg><Shape/></svg>}`,
    );
    try {
      assert.equal(document.querySelector('path').namespaceURI, 'http://www.w3.org/2000/svg');
    } finally {
      v.dispose();
    }
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
  'component whose only slot condition renders no nodes',
  { todo: 'runtime 要求组件至少返回一个节点；需讨论空模板占位策略' },
  async () => {
    await behavior(`export function A(props){return <>{props.children&&<b>present</b>}</>}`, '');
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

test(
  'SVG context across compiler-generated If and For boundaries',
  { todo: 'namespace 与组件所有权上下文需解耦，超出本轮静态继承修复' },
  async () => {
    const v = await mount(
      vm +
        `const state=vm({on:true,items:[1]});export function A(){return <svg>{state.on&&<path/>}{state.items.map(item=><circle key={item}/>)}</svg>}`,
    );
    try {
      assert.equal(document.querySelector('path').namespaceURI, 'http://www.w3.org/2000/svg');
      assert.equal(document.querySelector('circle').namespaceURI, 'http://www.w3.org/2000/svg');
    } finally {
      v.dispose();
    }
  },
);

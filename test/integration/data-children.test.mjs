import assert from 'node:assert/strict';
import { test } from 'node:test';

import { compile, mount } from './harness.mjs';

for (const [name, declaration, call] of [
  ['object arrow', 'const config={children:node=>node.children};', 'config.children(node)'],
  [
    'callback called props',
    'const config={children:props=>props.children};',
    'config.children(node)',
  ],
  ['computed children', 'const config={children:node=>node["children"]};', 'config.children(node)'],
  [
    'block callback',
    'const config={children:node=>{return node.children}};',
    'config.children(node)',
  ],
  [
    'function callback',
    'const config={children:function(node){return node.children}};',
    'config.children(node)',
  ],
  [
    'object method',
    'const config={children(node){return node.children}};',
    'config.children(node)',
  ],
  ['standalone helper', 'function getChildren(node){return node.children}', 'getChildren(node)'],
  [
    'exported helper',
    'export function getChildren(node){return node.children}',
    'getChildren(node)',
  ],
]) {
  test(`F14 ordinary data children: ${name}`, async () => {
    const v = await mount(
      `${declaration}export const node={children:[{label:'child'}]};export const result=${call};export function A(){return <span>{result[0].label}</span>}`,
    );
    try {
      assert.equal(v.m.result, v.m.node.children);
      assert.equal(v.text(), 'child');
    } finally {
      v.dispose();
    }
  });
}
for (const [name, declaration] of [
  ['function', 'function Frame(input){return input.children}'],
  ['arrow', 'const Frame=input=>input.children;'],
  ['computed', 'function Frame(input){return input["children"]}'],
  ['optional call', 'function Frame(input){return input.children?.()}'],
  ['binding alias', 'const forward=input=>input.children;const Frame=forward;'],
]) {
  test(`F14 renamed component props retain slot forwarding: ${name}`, async () => {
    const v = await mount(
      `${declaration}export function A(){return <Frame><strong>provided</strong></Frame>}`,
    );
    try {
      assert.equal(document.querySelector('strong')?.textContent, 'provided');
    } finally {
      v.dispose();
    }
  });
}
test('F14 a map callback shadowing component props reads business children', async () => {
  const v = await mount(
    `function Frame(input){return <section>{[{children:'data'}].map(input=><span>{input.children}</span>)}{input.children}</section>}export function A(){return <Frame><strong>provided</strong></Frame>}`,
  );
  try {
    assert.equal(document.querySelector('span')?.textContent, 'data');
    assert.equal(document.querySelector('strong')?.textContent, 'provided');
  } finally {
    v.dispose();
  }
});
test('F14 exported forwarding components retain their standalone render contract', () => {
  const result = compile('export function Frame(input){return input.children}');
  assert.ok(result.code.includes('renderSlotFunction$jg$('));
});

for (const [name, source] of [
  ['named export', 'function Frame(input){return input.children}export {Frame}'],
  ['renamed export', 'const forward=input=>input.children;export {forward as Frame}'],
  ['default function', 'export default function(input){return input.children}'],
  ['default arrow', 'export default input=>input.children'],
  ['default binding', 'const forward=input=>input.children;export default forward'],
  [
    'bootstrap alias',
    'import {bootstrap as start} from "jinge";const forward=input=>input.children;start(forward,document.body)',
  ],
  [
    'hoisted bootstrap import',
    'const forward=input=>input.children;start(forward,document.body);import {bootstrap as start} from "jinge"',
  ],
]) {
  test(`F14 bare slot forwarding retains component context: ${name}`, () => {
    assert.ok(compile(source).code.includes('renderSlotFunction$jg$('));
  });
}

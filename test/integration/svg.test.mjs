import assert from 'node:assert/strict';
import { test } from 'node:test';

import { flush, mount } from './harness.mjs';
const SVG = 'http://www.w3.org/2000/svg';
const HTML = 'http://www.w3.org/1999/xhtml';
const expectNamespace = (selector, namespace = SVG) => {
  const nodes = document.querySelectorAll(selector);
  assert.ok(nodes.length, selector);
  for (const node of nodes) assert.equal(node.namespaceURI, namespace, selector);
};
test('SVG exclusive tags across independent components', async () => {
  const v = await mount(
    `function Shape(){return <g><title>shape</title><path d="M0 0L10 10"/><circle r="5"/></g>}export function A(){return <svg><Shape/></svg>}`,
  );
  try {
    expectNamespace('svg,g,title,path,circle');
  } finally {
    v.dispose();
  }
});
test('SVG exclusive tags across generated If/For stay reactive and keyed', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({on:true,items:[{id:1,x:10},{id:2,x:20}]});export function A(){return <svg>{state.on&&<path d="M0 0L10 10"/>}{state.items.map(item=><circle key={item.id} cx={item.x} r="5"/>)}</svg>}`,
  );
  try {
    expectNamespace('path,circle');
    const circles = [...document.querySelectorAll('circle')];
    v.m.state.on = false;
    v.m.state.items.reverse();
    await flush();
    assert.equal(document.querySelector('path'), null);
    assert.deepEqual([...document.querySelectorAll('circle')], circles.toReversed());
    v.m.state.on = true;
    v.m.state.items[0].x = 40;
    await flush();
    expectNamespace('path,circle');
    assert.equal(document.querySelector('circle').getAttribute('cx'), '40');
  } finally {
    v.dispose();
  }
});
test('SVG exclusive tags in default and named slots', async () => {
  const v = await mount(
    `function Frame(props){return <svg>{props.children}{props['slot:extra']}</svg>}export function A(){return <Frame slot:extra={<path d="M0 0L10 10"/>}><circle r="5"/></Frame>}`,
  );
  try {
    expectNamespace('svg,circle,path');
  } finally {
    v.dispose();
  }
});
test('standalone foreignObject switches children to HTML and nested SVG switches back', async () => {
  const v = await mount(
    `function Content(){return <foreignObject width="100" height="100"><div><a href="#">link</a><svg><circle r="5"/></svg></div></foreignObject>}export function A(){return <svg><Content/></svg>}`,
  );
  try {
    expectNamespace('svg,foreignObject,circle');
    expectNamespace('div,a', HTML);
  } finally {
    v.dispose();
  }
});
test('HTML, custom elements and ambiguous tags keep their existing default namespace', async () => {
  const v = await mount(
    `export function A(){return <div><a href="#">link</a><title>title</title><style/><script type="application/json"/><my-widget/></div>}`,
  );
  try {
    expectNamespace('div,a,title,style,script,my-widget', HTML);
  } finally {
    v.dispose();
  }
});

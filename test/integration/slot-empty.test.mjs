import assert from 'node:assert/strict';
import { test } from 'node:test';

import { flush, mount, rt } from './harness.mjs';
const present = (_props, host) => {
  const node = document.createTextNode('provided');
  host[rt.ROOT_NODES].push(node);
  return [node];
};
for (const [name, expression, hasSlot, empty, text] of [
  ['and absent', 'props.children&&<b>yes</b>', false, true, ''],
  ['and present', 'props.children&&<b>yes</b>', true, false, 'yes'],
  ['named and absent', 'props["slot:header"]&&<b>yes</b>', false, true, ''],
  ['named and present', 'props["slot:header"]&&<b>yes</b>', true, false, 'yes'],
  ['ternary empty false', 'props.children?<b>yes</b>:<></>', false, true, ''],
  ['ternary nonempty true', 'props.children?<b>yes</b>:<></>', true, false, 'yes'],
  ['ternary empty true', 'props.children?<></>:<b>fallback</b>', true, true, ''],
  ['ternary nonempty false', 'props.children?<></>:<b>fallback</b>', false, false, 'fallback'],
  ['ternary null false', 'props.children?<b>yes</b>:null', false, true, ''],
  ['ternary undefined true', 'props.children?undefined:<b>fallback</b>', true, true, ''],
  ['or null fallback', 'props.children||null', false, true, ''],
  ['nullish undefined fallback', 'props.children??undefined', false, true, ''],
  ['nested empty fragment', 'props.children?<><></></>:<b>fallback</b>', true, true, ''],
  ['or empty fallback', 'props.children||<></>', false, true, ''],
  ['or provided slot', 'props.children||<></>', true, false, 'provided'],
  ['nullish empty fallback', 'props.children??<></>', false, true, ''],
  ['nullish provided slot', 'props.children??<></>', true, false, 'provided'],
])
  test(`empty slot branch: ${name}`, async () => {
    const v = await mount(`export function A(props,hostCustom){return <>{${expression}}</>}`, {
      props: hasSlot ? { 'slot:default': present, 'slot:header': present } : {},
    });
    try {
      assert.equal(v.text(), text);
      if (empty) {
        assert.equal(document.body.childNodes.length, 1);
        const node = document.body.firstChild;
        assert.equal(node.nodeType, Node.COMMENT_NODE);
        assert.deepEqual(v.host[rt.ROOT_NODES], [node]);
      }
      rt.destroyComponent(v.host);
      assert.equal(document.body.childNodes.length, 0);
    } finally {
      v.dispose();
    }
  });
test('missing slot below DOM owns its comment through the parent element', async () => {
  const v = await mount(
    `export function A(props){return <section>{props.children&&<b>yes</b>}</section>}`,
  );
  try {
    const section = document.querySelector('section');
    assert.equal(section.childNodes.length, 1);
    assert.equal(section.firstChild.nodeType, Node.COMMENT_NODE);
    assert.deepEqual(v.host[rt.ROOT_NODES], [section]);
    rt.destroyComponent(v.host);
    assert.equal(document.body.childNodes.length, 0);
  } finally {
    v.dispose();
  }
});
test('empty slot component can leave and reenter an outer If', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({on:true});function Empty(props){return <>{props.children&&<b>yes</b>}</>}export function A(){return <main>{state.on?<Empty/>:<i>end</i>}</main>}`,
  );
  try {
    const first = document.querySelector('main').firstChild;
    assert.equal(first.nodeType, Node.COMMENT_NODE);
    v.m.state.on = false;
    await flush();
    assert.equal(document.querySelector('main').textContent, 'end');
    assert.equal(first.isConnected, false);
    v.m.state.on = true;
    await flush();
    assert.equal(document.querySelector('main').childNodes.length, 1);
    assert.equal(document.querySelector('main').firstChild.nodeType, Node.COMMENT_NODE);
    assert.notEqual(document.querySelector('main').firstChild, first);
  } finally {
    v.dispose();
  }
});
test('empty slot root can be replaced in both directions', async () => {
  const v = await mount(`export function A(props){return <>{props.children&&<b>yes</b>}</>}`);
  try {
    const first = document.body.firstChild;
    rt.replaceRenderFunctionComponent(v.host, v.m.A, undefined, { 'slot:default': present });
    assert.equal(v.text(), 'yes');
    assert.equal(first.isConnected, false);
    rt.replaceRenderFunctionComponent(v.host, v.m.A, undefined, {});
    assert.equal(document.body.childNodes.length, 1);
    assert.equal(document.body.firstChild.nodeType, Node.COMMENT_NODE);
    assert.deepEqual(v.host[rt.ROOT_NODES], [document.body.firstChild]);
  } finally {
    v.dispose();
  }
});

test('slot existence remains static without a dynamic slot watcher', async () => {
  const props = rt.vm({ 'slot:default': undefined });
  const v = await mount(`export function A(props){return <>{props.children&&<b>yes</b>}</>}`, {
    props,
  });
  try {
    const comment = document.body.firstChild;
    props['slot:default'] = present;
    await flush();
    assert.equal(document.body.firstChild, comment);
    assert.equal(comment.nodeType, Node.COMMENT_NODE);
    assert.equal(props[rt.VM_WATCHERS]?.size ?? 0, 0);
  } finally {
    v.dispose();
  }
});

test('keyed empty slot component roots move and are removed with their list items', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({items:[{id:1},{id:2},{id:3}]});function Empty(props){return <>{props.children&&<b>yes</b>}</>}export function A(){return <main>{state.items.map(item=><Empty key={item.id}/>)}</main>}`,
  );
  try {
    const main = document.querySelector('main');
    const comments = [...main.childNodes];
    assert.equal(comments.length, 3);
    assert.ok(comments.every((node) => node.nodeType === Node.COMMENT_NODE));
    v.m.state.items.reverse();
    await flush();
    assert.deepEqual([...main.childNodes], comments.toReversed());
    v.m.state.items.splice(1, 1);
    await flush();
    assert.equal(comments[1].isConnected, false);
    assert.deepEqual([...main.childNodes], [comments[2], comments[0]]);
    v.m.state.items.length = 0;
    await flush();
    assert.ok(comments.every((node) => !node.isConnected));
  } finally {
    v.dispose();
  }
});

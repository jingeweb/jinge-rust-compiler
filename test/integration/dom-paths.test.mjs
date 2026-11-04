import assert from 'node:assert/strict';
import { test } from 'node:test';

import { behavior, flush, mount } from './harness.mjs';
const options = `<option value="shanghai" selected>Shanghai</option><option value="beijing" selected>Beijing</option>`;
const selected = () =>
  [...document.querySelector('select').selectedOptions].map((option) => option.value);
for (const [name, attr, expected, multi] of [
  ['constant true', 'multiple', ['shanghai', 'beijing'], true],
  ['constant false', 'multiple={false}', ['beijing'], false],
  ['reactive true', 'multiple={state.multi}', ['shanghai', 'beijing'], true],
  ['reactive false', 'multiple={state.multi}', ['beijing'], false],
])
  test(`D01 select initial multiple: ${String(name)}`, async () => {
    const v = await mount(
      `import {vm} from 'jinge';export const state=vm({multi:${String(multi)}});export function A(){return <select ${String(attr)}>${options}</select>}`,
    );
    try {
      assert.equal(document.querySelector('select').multiple, multi);
      assert.deepEqual(selected(), expected);
    } finally {
      v.dispose();
    }
  });
test('D01 select value initializes after options and continues responding', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({value:'beijing'});export function A(){return <select value={state.value}><option value="shanghai">Shanghai</option><option value="beijing">Beijing</option></select>}`,
  );
  try {
    assert.equal(document.querySelector('select').value, 'beijing');
    v.m.state.value = 'shanghai';
    await flush();
    assert.equal(document.querySelector('select').value, 'shanghai');
  } finally {
    v.dispose();
  }
});
test('D01 multiple precedes For children and later value assignment', async () => {
  const v = await mount(
    `import {vm} from 'jinge';const cities=vm(['shanghai','beijing']);export function A(){return <select multiple value="beijing">{cities.map(city=><option key={city} value={city} selected>{city}</option>)}</select>}`,
  );
  try {
    assert.equal(document.querySelector('select').multiple, true);
    assert.deepEqual(selected(), ['beijing']);
  } finally {
    v.dispose();
  }
});
test('D01 reactive multiple can change without reconstructing select', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({multi:true});export function A(){return <select multiple={state.multi}>${options}</select>}`,
  );
  try {
    const select = document.querySelector('select');
    assert.deepEqual(selected(), ['shanghai', 'beijing']);
    v.m.state.multi = false;
    await flush();
    assert.equal(document.querySelector('select'), select);
    assert.equal(select.multiple, false);
    assert.equal(select.selectedOptions.length, 1);
    v.m.state.multi = true;
    await flush();
    assert.equal(select.multiple, true);
  } finally {
    v.dispose();
  }
});
for (const [initial, next, before, after] of [
  ['hello', 'Alice!', 5, 6],
  ['', 'x', 0, 1],
  ['你好', '世界啊', 2, 3],
  ['😀', 'a😀b', 2, 4],
])
  test(`D02 string length ${JSON.stringify(initial)} -> ${JSON.stringify(next)}`, async () => {
    await behavior(
      `import {vm} from 'jinge';export const state=vm({text:${JSON.stringify(initial)}});export function A(){return <p>{state.text.length}</p>}`,
      String(before),
      (m) => (m.state.text = next),
      String(after),
    );
  });
test('D02 string indexes keep their VM root and rebind dynamic keys', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({text:'ab',key:0});export function A(){return <p>{state.text[state.key]}</p>}`,
  );
  try {
    assert.equal(v.text(), 'a');
    v.m.state.key = 1;
    await flush();
    assert.equal(v.text(), 'b');
    v.m.state.text = 'xy';
    await flush();
    assert.equal(v.text(), 'y');
    v.m.state.key = 8;
    await flush();
    assert.equal(v.text(), 'undefined');
    v.m.state.text = '123456789';
    await flush();
    assert.equal(v.text(), '9');
  } finally {
    v.dispose();
  }
});
test('D02 optional path recovers from null/undefined and empty string', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({user:null});export function A(){return <p>{state.user?.name.length}</p>}`,
  );
  try {
    assert.equal(v.text(), 'undefined');
    for (const [value, expected] of [
      [{ name: 'hello' }, '5'],
      [{ name: '' }, '0'],
      [undefined, 'undefined'],
      [{ name: '😀' }, '2'],
      [null, 'undefined'],
    ]) {
      v.m.state.user = value;
      await flush();
      assert.equal(v.text(), expected);
    }
  } finally {
    v.dispose();
  }
});
test('D02 primitive reads update attributes and arithmetic expressions', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({name:'abc'});export function A(){return <input title={state.name.length} value={state.name.length+1}/ >}`,
  );
  try {
    const input = document.querySelector('input');
    assert.equal(input.title, '3');
    assert.equal(input.value, '4');
    v.m.state.name = '';
    await flush();
    assert.equal(input.title, '0');
    assert.equal(input.value, '1');
  } finally {
    v.dispose();
  }
});

test('F03 static hidden follows native boolean reflection without changing aria-hidden', async () => {
  const v = await mount(
    `export function A(){return <><div id="bare" hidden/><div id="yes" hidden={true}/><div id="no" hidden={false}/><div id="unset" hidden={undefined}/><div id="aria" aria-hidden={false}/></>}`,
  );
  try {
    for (const id of ['bare', 'yes']) {
      assert.equal(document.getElementById(id).hidden, true);
      assert.equal(document.getElementById(id).hasAttribute('hidden'), true);
    }
    for (const id of ['no', 'unset']) {
      assert.equal(document.getElementById(id).hidden, false);
      assert.equal(document.getElementById(id).hasAttribute('hidden'), false);
    }
    assert.equal(document.getElementById('aria').getAttribute('aria-hidden'), 'false');
  } finally {
    v.dispose();
  }
});
test('F03 reactive hidden toggles presence and undefined clears the native state', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({hidden:false});export function A(){return <div hidden={state.hidden} aria-hidden={state.hidden}/>}`,
  );
  try {
    const el = document.querySelector('div');
    for (const value of [false, true, false, undefined]) {
      v.m.state.hidden = value;
      await flush();
      assert.equal(el.hidden, !!value);
      assert.equal(el.hasAttribute('hidden'), !!value);
      assert.equal(el.getAttribute('aria-hidden'), value === undefined ? null : String(value));
    }
  } finally {
    v.dispose();
  }
});

test('F07 static IDL bindings use native property casing', async () => {
  const v = await mount(
    `export function A(){return <><input autoComplete="username" autoFocus readOnly/><video autoPlay/></>}`,
  );
  try {
    const input = document.querySelector('input');
    const video = document.querySelector('video');
    assert.equal(input.autocomplete, 'username');
    assert.equal(input.getAttribute('autocomplete'), 'username');
    assert.equal(input.autofocus, true);
    assert.equal(input.readOnly, true);
    assert.equal(video.autoplay, true);
    for (const name of ['autoComplete', 'autoFocus', 'readonly'])
      assert.equal(Object.hasOwn(input, name), false);
    assert.equal(Object.hasOwn(video, 'autoPlay'), false);
  } finally {
    v.dispose();
  }
});
test('F07 reactive IDL bindings update native properties without expando fields', async () => {
  const v = await mount(
    `import {vm} from 'jinge';export const state=vm({complete:'username',enabled:true});export function A(){return <><input autoComplete={state.complete} autoFocus={state.enabled} readOnly={state.enabled}/><video autoPlay={state.enabled}/></>}`,
  );
  try {
    const input = document.querySelector('input');
    const video = document.querySelector('video');
    for (const [complete, enabled] of [
      ['username', true],
      ['off', false],
      ['', true],
    ]) {
      v.m.state.complete = complete;
      v.m.state.enabled = enabled;
      await flush();
      assert.equal(input.autocomplete, complete);
      assert.equal(input.getAttribute('autocomplete'), complete);
      for (const [el, property, attribute] of [
        [input, 'autofocus', 'autofocus'],
        [input, 'readOnly', 'readonly'],
        [video, 'autoplay', 'autoplay'],
      ]) {
        assert.equal(el[property], enabled);
        assert.equal(el.hasAttribute(attribute), enabled);
      }
    }
    for (const name of ['autoComplete', 'autoFocus', 'readonly'])
      assert.equal(Object.hasOwn(input, name), false);
    assert.equal(Object.hasOwn(video, 'autoPlay'), false);
  } finally {
    v.dispose();
  }
});

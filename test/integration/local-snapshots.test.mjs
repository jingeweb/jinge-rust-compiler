import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { compile, root } from './harness.mjs';

for (const [name, declaration, content] of [
  ['text', 'const name=item.name;', '<p>{name}</p>'],
  ['later assignment', 'let name;name=item.name;', '<p>{name}</p>'],
  ['optional member', 'const name=item.user?.name;', '<p>{name}</p>'],
  ['DOM attribute', 'const name=item.name;', '<p title={name}/>'],
  ['component property', 'const name=item.name;', '<Child value={name}/>'],
  ['derived expression', 'const name=item.name; const label="Hi "+name;', '<p>{label}</p>'],
  ['destructuring', 'const {name}=item;', '<p>{name}</p>'],
  ['aliased object', 'const user=item.user;', '<p>{user.name}</p>'],
  ['index snapshot', 'const position=index+1;', '<p>{position}</p>'],
]) {
  test(`local snapshot warns: ${name}`, () => {
    const source = `import {vm} from 'jinge';const items=vm([{id:1,name:'Alice',user:{name:'Alice'}}]);function Child(props){return <p>{props.value}</p>}export function A(){return <>{items.map((item,index)=>{${declaration}return <div key={item.id}>${content}</div>})}</>}`;
    const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stderr, /局部变量.*失去.*响应/);
    assert.match(result.stderr, /完整书写/);
  });
}
for (const [name, source] of [
  [
    'component local',
    `import {vm} from 'jinge';export function A(){const state=vm({name:'Alice'});const name=state.name;return <p title={name}>{name}</p>}`,
  ],
  [
    'top-level local',
    `import {vm} from 'jinge';const state=vm({name:'Alice'});const {name}=state;export function A(){return <p>{name}</p>}`,
  ],
]) {
  test(`local snapshot warns: ${name}`, () => {
    const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stderr, /局部变量.*失去.*响应/);
    assert.match(result.stderr, /完整书写/);
  });
}
for (const [name, declaration, content] of [
  ['direct path', '', '<p>{item.name}</p>'],
  ['constant', 'const name="fixed";', '<p>{name}</p>'],
  [
    'shadowed name',
    'const name=item.name;',
    '<Child render={()=>{const name="fixed";return name}}/>',
  ],
  ['deferred direct read', 'const read=()=>item.name;', '<button on:click={()=>read()}/>'],
  ['unused snapshot', 'const name=item.name;', '<p>{item.name}</p>'],
]) {
  test(`local snapshot boundary: ${name}`, () => {
    const source = `import {vm} from 'jinge';const items=vm([{id:1,name:'Alice'}]);function Child(props){return <p/>}export function A(){return <>{items.map(item=>{${declaration}return <div key={item.id}>${content}</div>})}</>}`;
    const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stderr, /局部变量.*失去.*响应/);
  });
}
test('local key error recommends a complete key expression', () => {
  assert.throws(
    () =>
      compile(
        `import {vm} from 'jinge';const items=vm([{id:1}]);export function A(){return <>{items.map(item=>{const id=item.id;return <p key={id}/>})}</>}`,
      ),
    /key=\{item\.id\}/,
  );
});

test('local key suggestion uses the actual callback parameter', () => {
  assert.throws(
    () =>
      compile(
        `import {vm} from 'jinge';const items=vm([{id:1}]);export function A(){return <>{items.map(row=>{const id=row.id;return <p key={id}/>})}</>}`,
      ),
    /key=\{row\.id\}/,
  );
});
for (const [name, source] of [
  [
    'imported unknown',
    `import {state} from './state';const name=state.name;export function A(){return <p>{name}</p>}`,
  ],
  [
    'factory unknown',
    `const state=createState();const name=state.name;export function A(){return <p>{name}</p>}`,
  ],
  [
    'vm alias',
    `import {vm} from 'jinge';const state=vm({name:'Alice'});const alias=state;export function A(){return <p>{alias.name}</p>}`,
  ],
  [
    'lexical shadowing',
    `import {vm} from 'jinge';const state=vm({name:'Alice'});const name=state.name;export function A(){const name='fixed';return <p>{name}</p>}`,
  ],
])
  test(`local snapshot unknown/alias boundary: ${name}`, () => {
    const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stderr, /局部变量.*失去.*响应/);
  });

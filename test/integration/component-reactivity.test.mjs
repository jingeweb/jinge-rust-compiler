import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { root } from './harness.mjs';

function warnings(source) {
  const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stderr;
}
const snapshot = /局部变量.*失去.*响应/;
const assigned = /重新赋值.*不会.*更新模板/;
for (const [name, source] of [
  [
    'vm derived text',
    `import {vm} from 'jinge';export function A(){const state=vm({name:'Alice'});const label='Hi '+state.name;return <span>{label}</span>}`,
  ],
  [
    'vm destructuring at file scope',
    `import {vm} from 'jinge';const state=vm({name:'Alice'});const {name}=state;export function A(){return <span>{name}</span>}`,
  ],
  [
    'vm object alias',
    `import {vm} from 'jinge';export function A(){const state=vm({user:{name:'Alice'}});const user=state.user;return <span>{user.name}</span>}`,
  ],
  [
    'object shorthand derivation',
    `import {vm} from 'jinge';export function A(){const state=vm({name:'Alice'});const name=state.name;const display={name};return <span>{display.name}</span>}`,
  ],
  ['props member', `export function A(props){const name=props.name;return <span>{name}</span>}`],
  [
    'props destructuring',
    `export function A(props){const {name}=props;return <span title={name}>{name}</span>}`,
  ],
  [
    'props derived member',
    `export function A(props){const label='Hi '+props.name;return <span>{label}</span>}`,
  ],
  [
    'props through alias',
    `export function A(props){const data=props;const name=data.name;return <span>{name}</span>}`,
  ],
  [
    'arrow component props',
    `export const A=(props)=>{const name=props.name;return <span>{name}</span>}`,
  ],
  [
    'anonymous component props',
    `export default function(props){const name=props.name;return <span>{name}</span>}`,
  ],
  [
    'derived mutable local',
    `export function A(){let count=0;const label=count+1;return <button on:click={()=>count++}>{label}</button>}`,
  ],
])
  test(`component snapshot diagnostic: ${name}`, () => {
    const output = warnings(source);
    assert.match(output, snapshot);
    assert.match(output, /完整书写/);
  });
for (const [name, declarations, update] of [
  ['assignment', 'let count=0;', 'count=1'],
  ['self-read assignment', 'let count=0;', 'count=count+1'],
  ['compound assignment', 'let count=0;', 'count+=1'],
  ['postfix update', 'let count=0;', 'count++'],
  ['prefix update', 'let count=0;', '++count'],
  ['var update', 'var count=0;', 'count++'],
  ['object destructuring assignment', 'let count=0;', '({count}={count:1})'],
  ['array destructuring assignment', 'let count=0;', '[count]=[1]'],
  ['for-of assignment', 'let count=0;', '(()=>{for(count of [1,2]){}})()'],
  ['for-in assignment', 'let count="";', '(()=>{for(count in {a:1}){}})()'],
])
  test(`component reassignment diagnostic: ${name}`, () => {
    const output = warnings(
      `export function A(){${declarations}return <button title={count} on:click={()=>{${update}}}>{count}</button>}`,
    );
    assert.match(output, assigned);
  });
for (const [name, source] of [
  [
    'assignment before declaration visitation',
    `function set(){count=1}let count=0;export function A(){return <button on:click={set}>{count}</button>}`,
  ],
  [
    'top-level event update',
    `let count=0;export function A(){return <button on:click={()=>count++}>{count}</button>}`,
  ],
  [
    'component property',
    `function Child(props){return <span>{props.value}</span>}export function A(){let count=0;return <Child value={count} on:change={()=>count++}/>}`,
  ],
  [
    'VM root rebind',
    `import {vm} from 'jinge';export function A(){let state=vm({name:'Alice'});return <button on:click={()=>state=vm({name:'Bob'})}>{state.name}</button>}`,
  ],
])
  test(`reassignment boundary diagnostic: ${name}`, () => assert.match(warnings(source), assigned));
for (const [name, source] of [
  ['unused let', `export function A(){let count=0;return <span>{count}</span>}`],
  ['constant', `const name='Alice';export function A(){return <span>{name}</span>}`],
  [
    'VM member updates',
    `import {vm} from 'jinge';export function A(){const state=vm({name:'Alice'});return <button on:click={()=>state.name='Bob'}>{state.name}</button>}`,
  ],
  [
    'let VM without rebinding',
    `import {vm} from 'jinge';export function A(){let state=vm({name:'Alice'});return <button on:click={()=>state.name='Bob'}>{state.name}</button>}`,
  ],
  ['direct props', `export function A(props){return <span>{props.name}</span>}`],
  [
    'props deferred event read',
    `export function A(props){return <button on:click={()=>console.log(props.name)}>read</button>}`,
  ],
  [
    'event only local',
    `export function A(){let count=0;return <button on:click={()=>count++}>change</button>}`,
  ],
  [
    'VM deferred read',
    `import {vm} from 'jinge';export function A(){const state=vm({name:'Alice'});const read=()=>state.name;return <button on:click={()=>console.log(read())}>read</button>}`,
  ],
  [
    'lexical shadowing',
    `let count=0;function update(){count++}export function A(){const count=7;return <button on:click={update}>{count}</button>}`,
  ],
  [
    'unknown external',
    `import {state} from './state';const name=state.name;export function A(){return <span>{name}</span>}`,
  ],
  [
    'ordinary helper parameter',
    `function helper(input){const name=input.name;return name}export function A(){const label=helper({name:'Alice'});return <span>{label}</span>}`,
  ],
])
  test(`component reactivity no warning: ${name}`, () => {
    const output = warnings(source);
    assert.doesNotMatch(output, snapshot);
    assert.doesNotMatch(output, assigned);
  });

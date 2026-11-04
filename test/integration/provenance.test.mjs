import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

import { root } from './harness.mjs';

function warnings(source) {
  const result = spawnSync(process.execPath, [`${root}/test/integration/warnings.mjs`, source], {
    encoding: 'utf8',
    timeout: 10000,
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stderr;
}
const snapshot = /局部变量 label .*失去.*响应/;
for (const [name, initialize, read] of [
  ['direct VM', `const state=vm({name:'Ada'});`, 'state.name'],
  ['alias chain', `const state=vm({name:'Ada'});const alias=state;const last=alias;`, 'last.name'],
  ['import alias', `const state=makeVM({name:'Ada'});`, 'state.name'],
  ['typed VM', `const state=vm({name:'Ada'}) as {name:string};`, 'state.name'],
]) {
  test(`S08 forward source diagnostics: ${name}`, () => {
    const render = `function Counter(){const label=${read};return <span>{label}</span>}`;
    const prefix = `import {vm,vm as makeVM} from 'jinge';`;
    const suffix = `export function A(){return <Counter/>}`;
    for (const source of [
      prefix + render + initialize + suffix,
      prefix + initialize + render + suffix,
    ]) {
      const output = warnings(source);
      assert.match(output, snapshot, source);
      assert.equal(
        (output.match(/局部变量 label .*失去.*响应/g) ?? []).length,
        1,
        'resolution must not duplicate warnings',
      );
    }
  });
}
test('S08 propagates through deferred nested declarations and transitive snapshots', () => {
  const output = warnings(
    `import {vm} from 'jinge';function Counter(){const first=state.name;const next=first;const label='Hello '+next;return <span>{label}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  );
  assert.match(output, snapshot);
});
test('S08 destructuring forward VM properties is a snapshot', () => {
  assert.match(
    warnings(
      `import {vm} from 'jinge';function Counter(){const {name:label}=state;return <span>{label}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
    ),
    snapshot,
  );
});
for (const [name, source] of [
  [
    'shadowing',
    `import {vm} from 'jinge';function Counter(){const state={name:'fixed'};const label=state.name;return <span>{label}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  ],
  [
    'unknown factory',
    `function Counter(){const label=state.name;return <span>{label}</span>}const state=createState();export function A(){return <Counter/>}`,
  ],
  [
    'unknown import',
    `import {state} from './state';function Counter(){const label=state.name;return <span>{label}</span>}export function A(){return <Counter/>}`,
  ],
  [
    'deferred closure',
    `import {vm} from 'jinge';function Counter(){const read=()=>state.name;return <button on:click={()=>read()}>read</button>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  ],
  [
    'deferred getter',
    `import {vm} from 'jinge';function Counter(){const label={get name(){return state.name}};return <span>{label.name}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  ],
  [
    'stable VM alias',
    `import {vm} from 'jinge';function Counter(){const alias=state;return <span>{alias.name}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  ],
  [
    'vm construction',
    `import {vm} from 'jinge';function Counter(){const local=vm({name:state.name});return <span>{local.name}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  ],
  [
    'cyclic aliases',
    `function Counter(){const a=b;const b=a;const label=a;return <span>{label}</span>}export function A(){return <Counter/>}`,
  ],
])
  test(`S08 no fabricated snapshot: ${name}`, () => {
    assert.doesNotMatch(warnings(source), /局部变量 .*失去.*响应/);
  });
test('S08 plain source diagnosis is independent of function placement', () => {
  for (const prefix of [true, false]) {
    const render = `function Counter(){return <span>{state.name}</span>}`;
    const declaration = `const state={name:'fixed'};`;
    assert.match(
      warnings(
        (prefix ? declaration + render : render + declaration) +
          `export function A(){return <Counter/>}`,
      ),
      /state 未使用 vm 包装/,
    );
  }
});
test('S08 writable roots keep reassignment warnings and uncertain provenance', () => {
  const output = warnings(
    `import {vm} from 'jinge';function Counter(){const label=state.name;return <button on:click={()=>state={name:'plain'}}>{label}</button>}let state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  );
  assert.match(output, snapshot);
  assert.doesNotMatch(output, /state 未使用 vm 包装/);
});

test('S08 shorthand references propagate delayed snapshot provenance', () => {
  assert.match(
    warnings(
      `import {vm} from 'jinge';function Counter(){const name=state.name;const label={name};return <span>{label.name}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
    ),
    snapshot,
  );
});
test('S08 computed getter keys are eager but getter bodies are deferred', () => {
  const eager = warnings(
    `import {vm} from 'jinge';function Counter(){const label={get [state.name](){return 'value'}};return <span>{label.Ada}</span>}const state=vm({name:'Ada'});export function A(){return <Counter/>}`,
  );
  assert.match(eager, snapshot);
  const deferred = warnings(
    `import {vm} from 'jinge';const state=vm({name:'Ada'});function Counter(){const label={get name(){return state.name}};return <span>{label.name}</span>}export function A(){return <Counter/>}`,
  );
  assert.doesNotMatch(deferred, snapshot);
});
test('S08 multiple var initializers stay uncertain instead of oscillating', () => {
  const output = warnings(
    `import {vm} from 'jinge';function Counter(){const label=state.name;return <span>{label}</span>}var state=vm({name:'Ada'});var state={name:'fixed'};export function A(){return <Counter/>}`,
  );
  assert.doesNotMatch(output, /state 未使用 vm 包装/);
});

test('S08 long stable alias chains terminate and reach the forward reader', () => {
  const length = 800;
  const aliases = Array.from(
    { length },
    (_, index) => `const a${index}=${index === 0 ? 'state' : `a${index - 1}`};`,
  ).join('');
  const source = `import {vm} from 'jinge';function Counter(){const label=a${length - 1}.name;return <span>{label}</span>}const state=vm({name:'Ada'});${aliases}export function A(){return <Counter/>}`;
  assert.match(warnings(source), snapshot);
});

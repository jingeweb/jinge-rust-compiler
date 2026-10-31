import { readFileSync } from 'node:fs';
// These inputs are both executable fixtures and reviewable codegen snapshots.

export const fixtures = {
  'text-fragment': { text: 'hello 世界42nulltrueundefined' },
  'dom-attributes': { text: 'ok' },
  svg: { text: 'html' },
  path: { text: 'a', mutate: (m) => (m.state.user = { name: 'b' }), after: 'b' },
  not: { text: 'true:false', mutate: (m) => (m.state.on = true), after: 'false:true' },
  expression: {
    text: '7',
    mutate: (m) => {
      m.state.n = 4;
      m.state.m = 1;
    },
    after: '9',
  },
  'template-literal': { text: 'Hello a', mutate: (m) => (m.state.name = 'b'), after: 'Hello b' },
  'dynamic-path': { text: 'A', mutate: (m) => (m.state.key = 'b'), after: 'B' },
  'optional-path': { text: 'undefined', mutate: (m) => (m.state.user = { name: 'b' }), after: 'b' },
  conditional: { text: '1', mutate: (m) => (m.state.on = false), after: 'off' },
  'logical-and': { text: '', mutate: (m) => (m.state.on = true), after: 'on' },
  'logical-or': { text: 'off', mutate: (m) => (m.state.on = true), after: 'true' },
  nullish: { text: 'fallback', mutate: (m) => (m.state.value = 'value'), after: 'value' },
  component: { text: '1', mutate: (m) => (m.state.n = 2), after: '2' },
  'nested-object-props': { text: '1', mutate: (m) => (m.state.n = 2), after: '2' },
  'component-spread': { text: '1', mutate: (m) => (m.state.n = 2), after: '2' },
  'default-slot': { text: 'a', mutate: (m) => (m.state.name = 'b'), after: 'b' },
  'named-slot': { text: 'headbody' },
  'scoped-slot': { text: '1', mutate: (m) => (m.state.n = 2), after: '2' },
  'slot-pass': { text: 'passed' },
  'slot-fallback': { text: 'fallback' },
  'slot-optional-call': { text: 'fallback' },
  'map-key-data': { text: 'ab', mutate: (m) => m.state.items.push('c'), after: 'abc' },
  'map-key-index': { text: '0:a;1:b;', mutate: (m) => m.state.items.shift(), after: '0:b;' },
  'map-key-path': { text: 'ab', mutate: (m) => m.state.items.reverse(), after: 'ba' },
  'map-block': { text: 'Hi a', mutate: (m) => (m.state.items[0].name = 'b'), after: 'Hi b' },
  'map-nested': {
    text: 'a',
    mutate: (m) => m.state.groups[0].items.push({ id: 3, name: 'b' }),
    after: 'ab',
  },
  'event-static': { text: '0', event: 'click', after: '1' },
  'event-dynamic': { text: '0', event: 'click', after: '1' },
  'event-expression': { text: '0', event: 'click', after: '1' },
  'event-component': { text: 'go0', event: 'click', after: 'go3' },
  ref: { text: 'ref' },
  'intl-text': { intl: 1, text: 'hello' },
  'intl-params': { intl: 1, text: 'Hi a', mutate: (m) => (m.state.name = 'b'), after: 'Hi b' },
  'intl-attribute': { intl: 1, text: 'x', mutate: (m) => (m.state.name = 'b'), after: 'x' },
  'object-slots': { text: 'root' },
};

Object.assign(fixtures, {
  'map-fragment-key': { text: 'a!' },
  'map-string-key': { text: 'a' },
  'map-external-key': { text: 'a' },
  'map-computed-prefix-key': { text: 'a' },
  'map-no-params': { text: 'x' },
  'slot-function': { text: 'Hi a' },
  'slot-and': { text: '' },
  'slot-or': { text: 'fallback' },
  'slot-ternary': { text: 'missing' },
  'component-event-path': { text: '0', event: 'click', after: '1' },
  'ref-function': { text: 'ref' },
  'intl-const-params': { intl: 1, text: 'Hi a' },
  'intl-drop-fallback': { intl: 2, text: 'missing' },
  'object-slot-function': { text: 'root' },
  'component-arrow': { text: 'arrow' },
});

for (const [name, fixture] of Object.entries(fixtures))
  fixture.source = readFileSync(
    new URL(`../rust/fixtures/${name}/input.tsx.txt`, import.meta.url),
    'utf8',
  );

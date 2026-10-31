import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';

import { fixtures } from './fixtures.mjs';
import { compile, flush, mount, root, rt } from './harness.mjs';
const snapshots = path.join(root, 'test/rust/fixtures');
await rt.intlInit({
  defaultLocale: 'fixtures',
  dicts: { fixtures: { 'fixture-params': (p) => `Hi ${p.name}` } },
});
for (const [name, fixture] of Object.entries(fixtures))
  test(`codegen and execution: ${name}`, async () => {
    const result = compile(fixture.source, { intl: fixture.intl });
    const snapshot = path.join(snapshots, name, 'expected.js');
    assert.equal(result.code, readFileSync(snapshot, 'utf8'), name);
    assert.ok(!result.code.includes('<>'));
    const view = await mount(fixture.source, { intl: fixture.intl });
    try {
      assert.equal(view.text(), fixture.text);
      if (fixture.mutate) {
        fixture.mutate(view.m);
        await flush();
        assert.equal(view.text(), fixture.after);
      }
      if (fixture.event) {
        document
          .querySelector('button')
          .dispatchEvent(new window.MouseEvent(fixture.event, { bubbles: true }));
        await flush();
        assert.equal(view.text(), fixture.after);
      }
      if (name === 'ref') assert.equal(view.m.el.value, document.querySelector('p'));
      if (name === 'intl-attribute') assert.equal(document.querySelector('p').title, 'Hi b');
    } finally {
      view.dispose();
    }
  });

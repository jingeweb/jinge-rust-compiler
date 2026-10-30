import { spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync, copyFileSync } from 'node:fs';
// 临时审查探针：先 cargo build --locked；依赖同级 jinge 的 Jest/jsdom 和 dist 构建。
// 记录实际结果和期望，不作为通过/失败门禁；输出保存在系统临时目录 jinge-compiler-review。
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { runInNewContext } from 'node:vm';
const compilerRoot = path.resolve(import.meta.dirname, '../..');
const root = path.dirname(compilerRoot);
const dir = path.join(os.tmpdir(), 'jinge-compiler-review');
mkdirSync(dir, { recursive: true });
if (process.argv[2] !== 'worker') {
  const binary =
    process.platform === 'darwin'
      ? 'libjinge_compiler.dylib'
      : process.platform === 'win32'
        ? 'jinge_compiler.dll'
        : 'libjinge_compiler.so';
  copyFileSync(path.join(compilerRoot, 'target/debug', binary), path.join(dir, 'compiler.node'));
}
const runtimeRequire = createRequire(`${root}/jinge/package.json`);
const ts = createRequire(`${root}/jinge-compiler/package.json`)('@typescript/typescript6');
const cases = {
  const_checked_false: {
    source: `export function A(){return <input checked={false} disabled={false}/>}`,
    expected: '',
    domProps: ['checked', 'disabled'],
  },
  const_value_input: {
    source: `export function A(){return <input value="hello"/>}`,
    expected: '',
    domProps: ['value'],
  },
  generic_ts: { source: `export const identity=<T>(x:T):T=>x;`, type: 1, compileOnly: true },
  generic_assertion_ts: { source: `export const n=<number>123;`, type: 1, compileOnly: true },
  optslot_undefined: {
    source: `export function A(props){return <>{props['slot:content']?.() ?? <b>empty</b>}</>}`,
    expected: 'empty',
  },

  unicode_silent: {
    source: 'export function A(){return <div>\n你好世\n</div>}',
    expected: '你好世',
  },
  import_only_slot: {
    source: `export const opts={'slot:content': <p>hi</p>};`,
    expected: 'hi',
    slotOnly: true,
  },
  method_readonly: {
    source: `import {vm} from 'jinge';export const state=vm({n:1,user:{n:7,get(){return this.n}}});export function A(){return <div>{state.user.get()+state.n}</div>}`,
    expected: '8',
  },
  nested_object_binding: {
    source: `import {vm} from 'jinge';export const state=vm({n:1});function Child(props){return <p>{props.options.deep.n}</p>};export function A(){return <Child options={{deep:{n:state.n}}}/>}`,
    expected: '1',
    mutate: `m.state.n=2`,
    after: '2',
  },
  intl_attr_params_update: {
    source: `import {vm,t} from 'jinge';export const state=vm({name:'Alice'});export function A(){return <div>{t('Hello {name}',{name:state.name})}</div>}`,
    intl: 1,
    expected: 'Hello Alice',
    dict: true,
    mutate: `m.state.name='Bob'`,
    after: 'Hello Bob',
  },
  intl_attr_params_update2: {
    source: `import {vm,t} from 'jinge';export const state=vm({name:'Alice'});export function A(){return <div title={t('Hello {name}',{name:state.name})}/>}`,
    intl: 1,
    expected: '',
    dict: true,
    mutate: `m.state.name='Bob'`,
    after: '',
    attr: ['title', 'Hello Alice'],
    attrAfter: 'Hello Bob',
  },
  intl_rich_fallback: {
    source: `import {t} from 'jinge';export function A(){return <div>{t('Hello <B>world</B>',{B:(text)=><b>{text}</b>})}</div>}`,
    intl: 1,
    expected: 'Hello <B>world</B>',
  },
  optional_call: {
    source: `import {vm} from 'jinge';export const state=vm({fn:()=> 'ok'});export function A(){return <div>{state.fn?.()}</div>}`,
    expected: 'ok',
  },
  optional_chain_lazy: {
    source: `import {vm} from 'jinge';export const state=vm({obj:null,n:0});export function A(){return <div>{state.obj?.[state.n++]}</div>}`,
    expected: 'undefined',
    compileOnly: true,
  },
  custom_host_slot_cond: {
    source: `export function A(props,hostCustom){return <>{props.children ? <p>yes</p> : <p>no</p>}</>}`,
    expected: 'no',
  },
  nested_slot_children: {
    source: `function Child(props){return <div>{props.children({label:'ok'})}</div>};export function A(props){return <Child>{slotProps=><p>{slotProps.label}{props.children ?? <b>fallback</b>}</p>}</Child>}`,
    expected: 'okfallback',
  },
  reactivity_ternary_guard: {
    source: `import {vm} from 'jinge';export const state=vm({on:false,user:null});export function A(){return <div>{state.on ? state.user.name : 'no'}</div>}`,
    expected: 'no',
  },
  scope_reused_props: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{name:'outer'}]});export function A(){return <div>{state.items.map(item=><p key={item}>{((item)=>item.name)({name:'inner'})}</p>)}</div>}`,
    expected: 'inner',
  },

  unicode_multiline: {
    source: 'export function A(){return <div>\n你好\n世界\n</div>}',
    expected: '你好 世界',
  },
  unicode_truncation: {
    source: 'export function A(){return <div>\n你好世界abc\n</div>}',
    expected: '你好世界abc',
  },
  method_this: {
    source: `import {vm} from 'jinge'; export const state=vm({user:{n:7,get(){return this.n}}}); export function A(){return <div>{state.user.get()}</div>}`,
    expected: '7',
  },
  string_method: {
    source: `import {vm} from 'jinge';export const state=vm({name:'alice'});export function A(){return <div>{state.name.toUpperCase()}</div>}`,
    expected: 'ALICE',
  },
  callback_scope: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{active:true}]});export function A(){return <div>{state.items.some(item=>item.active)}</div>}`,
    expected: 'true',
  },
  map_identity: {
    source: `import {vm} from 'jinge';export const state=vm({items:[1,2]});export function A(){return <div>{state.items.map(item=>item)}</div>}`,
    expected: '12',
  },
  map_shadow: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{name:'outer'}]});export function A(){return <div>{state.items.map(item=>{const f=()=>{const item={name:'inner'};return item.name};return <p key={item}>{f()}</p>})}</div>}`,
    expected: 'inner',
  },
  map_nested_arrow: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{name:'outer'}]});export function A(){return <div>{state.items.map(item=><p key={item}>{[1].map(()=>item)}</p>)}</div>}`,
    expected: '{"name":"outer"}',
  },
  map_destructure: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{name:'outer'}]});export function A(){return <div>{state.items.map(({name})=><p>{name}</p>)}</div>}`,
    expected: 'outer',
  },
  key_path: {
    source: `import {vm} from 'jinge';export const state=vm({items:[{meta:{id:1}}]});export function A(){return <div>{state.items.map(item=><p key={item.meta['id']}>ok</p>)}</div>}`,
    expected: 'ok',
  },
  component_attr_hyphen: {
    source: `import {vm} from 'jinge';export const state=vm({n:'hi'});function Child(props){return <p>{props['data-id']}</p>};export function A(){return <Child data-id={state.n}/>}`,
    expected: 'hi',
  },
  spread_dom: {
    source: `export function A(){const attrs={id:'hello',title:'world'};return <div {...attrs}>ok</div>}`,
    expected: 'ok',
    attr: ['id', 'hello'],
  },
  spread_slots: {
    source: `function Child(props){return <div>{props.children}</div>};export function A(){const attrs={};return <Child {...attrs}><p>hello</p></Child>}`,
    expected: 'hello',
  },
  svg_namespace: {
    source: `export function A(){return <svg><g><path/></g></svg>}`,
    expected: '',
    svg: true,
  },
  early_return: {
    source: `export function A(props){if(props.ok)return <p>yes</p>;return <p>no</p>}`,
    expected: 'yes',
    props: { ok: true },
  },
  trailing_statement: { source: `export function A(){return <p>yes</p>; ;}`, expected: 'yes' },
  object_slot_import: {
    source: `export const opts={'slot:content': <p>hi</p>};export function A(){return opts['slot:content']}`,
    expected: 'hi',
  },
  object_slot_optional: {
    source: `export const opts={'slot:content': value?.['slot:a']};export function A(){return <p>hi</p>}`,
    compileOnly: true,
  },
  syntax_error: { source: `export function A(){return <div>}`, compileOnly: true },
  diagnostic_return: {
    source: `export function A(){return <><div {...{id:'x'}}>hello</div></>}`,
    compileOnly: true,
  },
  intl_ts: {
    source: `import {t} from 'jinge';export const label=t('hello');`,
    type: 1,
    intl: 2,
    compileOnly: true,
  },
  intl_two_calls: {
    source: `import {t} from 'jinge';export function A(){return <div title={t('hello')+t('world')}/ >}`,
    intl: 1,
    compileOnly: true,
  },
  intl_mixed_calls: {
    source: `import {t} from 'jinge';const format=(x)=>'['+x+']';export function A(){return <div title={format('x')+t('hello')}/>}`,
    intl: 1,
    expected: '',
    attr: ['title', '[x]hello'],
  },
  intl_params: {
    source: `import {vm,t} from 'jinge';export const state=vm({name:'Alice'});export function A(){return <div title={t('Hello {name}',{name:state.name})}/>}`,
    intl: 1,
    expected: '',
    attr: ['title', 'Hello Alice'],
    dict: true,
  },
  static_key: {
    source: `import {vm} from 'jinge';export const state=vm({name:'Alice'});const key='name';export function A(){return <div>{state[key]}</div>}`,
    expected: 'Alice',
    mutate: `m.state.name='Bob'`,
    after: 'Bob',
  },
  watch_after_method: {
    source: `import {vm} from 'jinge';export const state=vm({items:[1,2]});export function A(){return <div>{state.items.join(',')}</div>}`,
    expected: '1,2',
    mutate: `m.state.items.push(3)`,
    after: '1,2,3',
  },
  field_nested_prop: {
    source: `import {vm} from 'jinge';export const state=vm({n:1});function Child(props){return <p>{props.options.n}</p>};export function A(){return <Child options={{n:state.n}}/>}`,
    expected: '1',
    mutate: `m.state.n=2`,
    after: '2',
  },
  scoped_hmr: {
    source: `function factory(){function Inner(){return <p>hello</p>};return Inner;}export const A=factory();`,
    expected: 'hello',
    hmr: true,
  },
  id_collision: {
    source: `import {vm} from 'jinge';const a0=10;export const state=vm({n:1});export function A(){return <div>{a0+state.n}</div>}`,
    expected: '11',
  },
};
if (process.argv[2] === 'worker') {
  const name = process.argv[3],
    c = cases[name];
  const binding = createRequire(import.meta.url)(`${dir}/compiler.node`);
  try {
    const result = binding.transform(
      `${name}.${c.type === 1 ? 'ts' : 'tsx'}`,
      c.type ?? 2,
      c.source,
      false,
      c.intl ?? 0,
    );
    writeFileSync(`${dir}/${name}.input.tsx`, c.source);
    writeFileSync(`${dir}/${name}.output.js`, result.code);
    const parsed = ts.createSourceFile(
      'out.js',
      result.code,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.JS,
    );
    const output = {
      name,
      components: result.parsedComponents,
      mapWhenDisabled: !!result.map,
      syntaxErrors: parsed.parseDiagnostics.map((d) =>
        ts.flattenDiagnosticMessageText(d.messageText, ' '),
      ),
    };
    if (!c.compileOnly) {
      const { JSDOM } = createRequire(runtimeRequire.resolve('jest-environment-jsdom'))('jsdom');
      const dom = new JSDOM('<!doctype html><body></body>', { url: 'http://localhost/' });
      for (const k of [
        'window',
        'document',
        'Node',
        'Element',
        'HTMLElement',
        'SVGElement',
        'Text',
        'Comment',
        'Event',
        'DocumentFragment',
      ])
        globalThis[k] = dom.window[k];
      globalThis.OffscreenCanvas = class {};
      const runtimeUrl = pathToFileURL(`${root}/jinge/dist/jinge.dev.js`).href;
      const rt = await import(runtimeUrl);
      if (c.dict) {
        const key = createRequire(`${root}/jinge-compiler/package.json`)('node:crypto')
          .createHash('sha512')
          .update('Hello {name}')
          .digest('base64')
          .slice(0, 6);
        await rt.intlInit({
          defaultLocale: 'en',
          dicts: { en: { [key]: (p) => `Hello ${p.name}` } },
        });
      }
      const js = result.code.replaceAll(/(['"])jinge\1/g, JSON.stringify(runtimeUrl));
      const m = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
      const host = new rt.ComponentHost();
      const nodes = c.slotOnly
        ? rt.renderSlotFunction(host, m.opts['slot:content'], {})
        : rt.renderFunctionComponent(host, m.A, c.props ?? {});
      document.body.append(...nodes);
      rt.handleRenderDone(host);
      output.actual = document.body.textContent;
      output.expected = c.expected;
      if (c.attr) {
        output.attrActual = document.body.firstElementChild?.getAttribute(c.attr[0]);
        output.attrExpected = c.attr[1];
      }
      if (c.domProps)
        output.domProps = Object.fromEntries(
          c.domProps.map((k) => [k, document.body.firstElementChild[k]]),
        );
      if (c.svg)
        output.namespaces = [...document.querySelectorAll('*')]
          .filter((el) => ['svg', 'g', 'path'].includes(el.tagName.toLowerCase()))
          .map((el) => [el.tagName, el.namespaceURI]);
      if (c.mutate) {
        runInNewContext(c.mutate, { m });
        await new Promise((r) => setTimeout(r, 80));
        output.afterActual = document.body.textContent;
        output.afterExpected = c.after;
        if (c.attrAfter) {
          output.attrAfterActual = document.body.firstElementChild?.getAttribute(c.attr[0]);
          output.attrAfterExpected = c.attrAfter;
        }
      }
      rt.destroyComponent(host);
      dom.window.close();
    }
    console.log(JSON.stringify(output));
  } catch (e) {
    console.log(
      JSON.stringify({ name, error: e.message, stack: e.stack?.split('\n').slice(0, 1) }),
    );
  }
  process.exit(0);
} else {
  const results = [];
  for (const name of Object.keys(cases)) {
    const p = spawnSync(process.execPath, [import.meta.filename, 'worker', name], {
      encoding: 'utf8',
      timeout: 15000,
    });
    const lines = p.stdout?.trim().split('\n') ?? [];
    let data;
    try {
      data = JSON.parse(lines.at(-1));
    } catch {
      data = { name, exit: p.status, signal: p.signal, stdout: p.stdout };
    }
    // oxlint-disable-next-line no-control-regex -- 去掉编译诊断中的 ANSI 颜色转义。
    data.stderr = p.stderr?.replaceAll(/\x1b\[[0-9;]*m/g, '').trim();
    results.push(data);
    const summary = { ...data };
    delete summary.stack;
    delete summary.stderr;
    console.log(JSON.stringify(summary));
  }
  writeFileSync(`${dir}/results.json`, JSON.stringify(results, null, 2));
}

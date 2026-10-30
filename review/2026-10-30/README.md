# jinge-compiler 源码审查与测试方案

审查日期：2026-10-30。范围包括全部 Rust 源码、TS binding、Vite 插件、国际化提取/编译 CLI，以及它们与当前 jinge 运行时的接口。没有修改编译器实现；仓库已有的 README.md 改动保留。

以下区分实际误编译、需要明确约束的输入、测试与诊断系统缺口。不能把所有合法 TypeScript 都自动视为 Jinge 已承诺支持的模板语法；但明确不支持的语法也应报错，不能静默生成错误代码。

## 验证方式与当前基线

- `cargo build --locked` 通过。复现使用当前源码构建的本地 binding，不使用 npm 发布的二进制。
- `cargo test --locked`：2 个测试，1 个通过，1 个失败。`src/lib.rs` 的 `test_transform` 最后是 `assert!(false)`，属于调试用例，不是正常的回归测试。
- `pnpm lint` 通过。静态类型与 lint 检查不能发现下面的生成代码语义问题。
- `compiler-probes.mjs` 使用真实 binding 编译，将产物接入同级 jinge 的开发版运行时和 jsdom，检查首屏、修改数据后的结果、DOM 属性和 namespace。每个输入放入独立子进程，隔离 panic 与全局状态。
- `intl-probes.ts` 直接调用 TS 国际化工具，检查 CSV 往返、输出语法及执行结果。
- 探针记录实际结果与期望，**不是已经接入 CI 的正式测试系统**，也不是“所有用例应当通过”的测试集合。包括正常对照组和不支持语法的诊断探针。

在 compiler 目录运行：

```sh
cargo build --locked
node review/2026-10-30/compiler-probes.mjs
./node_modules/.bin/bun review/2026-10-30/intl-probes.ts
```

运行时探针要求同级 `../jinge` 已安装测试依赖并完成 `pnpm build`。临时产物位于系统临时目录下的 `jinge-compiler-review/`，包括输入、生成的 JS、`results.json`。此脚本是当前多仓库工作区的审查工具，正式 CI 应使用显式版本的运行时测试依赖。

## 优先处理的实际问题

### C01：多行非 ASCII 文本会 panic，或静默截断

位置：[parser/mod.rs](../../src/parser/mod.rs#L443)，`trim_html_text`。

```tsx
export function A() {
  return <div>你好 世界</div>;
}
```

循环通过 `chars().enumerate()` 得到字符序号，却拿该序号切 UTF-8 字节字符串。已复现 `not a char boundary` 的 Rust panic；`\n你好世\n` 这种恰好落在合法字节边界的文本只留下“你”。带换行的中文、emoji 都可能触发，单行文本因走另一分支看不出来。

建议使用 `char_indices` 或按行/字符处理，不混用字符与字节下标；同时保留原有空白规则。属于局部修复。

### C02：方法调用被改写为裸函数调用，丢失接收者和依赖语义

位置：[parser/expr.rs](../../src/parser/expr.rs#L175)、[MemberExprReplaceVisitor](../../src/parser/expr.rs#L271)。

```tsx
const state = vm({
  user: {
    n: 7,
    get() {
      return this.n;
    },
  },
});
return <div>{state.user.get()}</div>;
```

产物核心变为 `ExprWatcher([PathWatcher(state, ['user','get'], true)], a0 => a0())`，运行时报 `Cannot read properties of undefined (reading 'n')`。

`state.name.toUpperCase()` 也失败：属性路径求值不能沿字符串原始值继续取方法，结果为 undefined。数组 `join` 是正常对照组，因为运行时数组代理额外绑定了方法，不能据此认为所有方法调用都正确。

这里不能仅补 `.call`：方法体读取哪些字段、该监听谁，仍需明确规则。建议先支持可明确定位的接收者依赖，或者对未支持的方法调用给诊断；不要假装已有完整函数依赖追踪。

### C03：表达式中的回调参数被提到作用域外

位置：[parser/expr.rs](../../src/parser/expr.rs#L175)，默认遍历进入嵌套函数。

```tsx
return <div>{state.items.some((item) => item.active)}</div>;
```

生成 `PathWatcher(item, ['active'], true)`，它位于回调之外，首屏即 `ReferenceError: item is not defined`。数组回调、立即调用的箭头函数等都会遇到同类风险。

依赖提取必须识别词法绑定边界。可选择将回调当作不展开的子表达式，并监听外围集合/对象；若暂不支持，应明确报错。需要设计确认，不能只跳过所有函数后宣称依赖完整。

### C04：普通函数的可选调用会丢掉调用动作

位置：[parser/expr.rs](../../src/parser/expr.rs#L85)。

```tsx
const state = vm({ fn: () => 'ok' });
return <div>{state.fn?.()}</div>;
```

实际显示 `()=>'ok'`，而不是 `ok`。所有 `OptChain` 被当成简单成员路径，代码注释假设其 base 不会是 Call，但输入并不满足该假设。应区分可选成员与可选调用，并保留短路及参数求值语义。

### C05：生成的 a0 参数捕获了用户同名变量

位置：[parser/expr.rs](../../src/parser/expr.rs#L260)、[lib.rs](../../src/lib.rs#L141)。

```tsx
const a0 = 10;
const state = vm({ n: 1 });
return <div>{a0 + state.n}</div>;
```

生成 `(a0) => a0 + a0`，结果为 2，预期 11。生成标识符没有避免捕获，且 hygiene 被注释。需要统一生成标识符的唯一性与 SyntaxContext；仅改个更长的固定前缀不是完整修复。

### C06：国际化调用以 SyntaxContext 标记，误认同表达式中的其他函数调用

位置：[parser/expr.rs](../../src/parser/expr.rs#L230)、[替换逻辑](../../src/parser/expr.rs#L282)。

```tsx
const format = (x) => '[' + x + ']';
return <div title={format('x') + t('hello')} />;
```

实际 title 为 `helloundefined`，预期 `[x]hello`。`CallExpr.ctxt` 不是节点的唯一标识；把 t 调用的 ctxt 放入集合后，其他同 ctxt 调用也被替换，而 watcher 列表只添加 t 的项，造成参数错位。

应以具体节点位置/显式替换计划关联依赖，或在一次遍历中同时建立替换与 watcher，保证一一对应。

### C07：SVG 上下文只保留一层

位置：[parser/jsx.rs](../../src/parser/jsx.rs#L9)。

```tsx
return (
  <svg>
    <g>
      <path />
    </g>
  </svg>
);
```

产物为 `createSVGEle('svg', createSVGEle('g', createEle('path')))`。实测 path 的 namespace 是 `http://www.w3.org/1999/xhtml`。进入 g 时上下文被写成 Html，孙节点失去 SVG 语义。

可局部修复上下文继承，但还应单独验证 `foreignObject` 返回 HTML namespace 的边界，以及条件分支、组件/插槽穿越的 SVG 上下文策略。

### C08：带连字符的响应式组件属性生成非法赋值

位置：[parser/attrs.rs](../../src/parser/attrs.rs#L190)、[parser/tpl.rs](../../src/parser/tpl.rs#L304)。

```tsx
return <Child data-id={state.n} />;
```

生成 `attrs$jg$.data-id = v`，模块无法执行：`Invalid left-hand side in assignment`。常量属性正确使用字符串 key；响应式属性没有区分合法标识符。应改为 computed member，或统一字符串属性访问。属于局部修复。

### C09：常量布尔 DOM 属性 false 会变成 true

位置：[parser/attrs.rs](../../src/parser/attrs.rs#L145)、[parser/jsx.rs](../../src/parser/jsx.rs#L53)，以及运行时 `createEleA` 的属性设置约定。

```tsx
return <input checked={false} disabled={false} />;
```

实测 DOM 的 checked、disabled 均为 true，因为常量路径走 `setAttribute(..., false)`，产生存在的 HTML 布尔属性；响应式路径则对 IDL 属性赋值。常量和响应式路径行为不一致。

这是编译器/运行时接口共同的问题，应统一属性设置语义，并覆盖 checked、disabled、selected 等，不只修单个属性名。

### C10：spread 在 DOM 上静默丢失；在组件上吞掉 children

位置：[parser/attrs.rs](../../src/parser/attrs.rs#L362)、[parser/jsx.rs](../../src/parser/jsx.rs#L9)、[parser/component.rs](../../src/parser/component.rs#L167)。

```tsx
const attrs = { id: 'hello' };
return <div {...attrs}>ok</div>;
```

没有错误，id 也没有生成。DOM 解析保存 `spread_prop`，后续没有消费它。若 DOM spread 不是目标能力，应在解析处直接拒绝。

```tsx
const attrs = {};
return (
  <Child {...attrs}>
    <p>hello</p>
  </Child>
);
```

children 被生成到局部 attrs 容器，但调用 Child 时只传原始 spread 对象，最终 children 丢失。若保持“原样透传同一个响应式 props 对象”的设计，则 spread 与本地 slot 应明确互斥；若允许组合，必须讨论如何合并而不破坏响应式身份。

### C11：单独的对象字面量插槽转换后缺失运行时 import

位置：[visitor.rs](../../src/visitor.rs#L217)。

```tsx
export const opts = { 'slot:content': <p>hi</p> };
```

插槽已改成渲染函数，却没有设置 `self.changed = true`，模块不注入 jinge 的运行时导入。真实调用该 slot 报 `createEle$jg$ is not defined`。同文件有普通 JSX 组件时恰好被掩盖。可局部修复。

### C12：map 参数替换按名字匹配，会跨过局部变量遮蔽

位置：[parser/map.rs](../../src/parser/map.rs#L75)。

```tsx
state.items.map((item) => {
  const f = () => {
    const item = { name: 'inner' };
    return item.name;
  };
  return <p>{f()}</p>;
});
```

内层 `return item.name` 被改为 `return each$jg$0.data.name`。实测输出 outer，预期 inner。当前只检查部分函数参数遮蔽，忽略局部声明、块级作用域和解构绑定。

前面已经运行 SWC resolver，建议保留并使用绑定 Id（含上下文），不要退化为 Atom 名称匹配。

### C13：computed 属性中的普通变量导致整个表达式不再监听

位置：[parser/expr.rs](../../src/parser/expr.rs#L389)。

```tsx
const key = 'name';
return <div>{state[key]}</div>;
```

首屏 Alice；之后 `state.name = 'Bob'`，界面仍是 Alice。普通标识符索引不产生内部 watcher，被当成解析失败，进而整段降级为常量。没有任何诊断。

普通固定 key 可作为静态路径项处理，仍监听 state；若 key 是可变的普通局部变量，必须明确它本身不具有响应式能力。

### C14：map key 的混合路径拼接错误，块体回调的 key 被忽略

位置：[parser/map_key.rs](../../src/parser/map_key.rs#L82)、[get_key](../../src/parser/map_key.rs#L216)。

`key={item.meta['id']}` 被编码成 `meta.['id']`，运行时构建 keyFn 时抛 `Unexpected token '['`。应正确连接点访问和方括号访问；也应测试反斜杠、引号等字符串 key，避免把属性名拼成不正确的 JS 源码。

另一个已观察到的缺口：块体箭头函数 `item => { return <p key={item.id}/> }` 已被允许编译，但 `get_key` 直接返回 None；key 只留在 DOM 属性中，For 实际没有 key。此时发“缺少 key”警告也不准确。需要提取最终 return 的 key，或明确拒绝这一组合。

### C15：.ts 文件使用 TSX 解析模式，拒绝常规 TypeScript 语法

位置：[lib.rs](../../src/lib.rs#L85)。

```ts
export const identity = <T>(x: T): T => x;
export const n = <number>123;
```

使用 code_type=1、`.ts` 文件名分别编译，两者都触发 parse panic。Lexer 无条件 `tsx: true`。Vite serve 又关闭了 esbuild/oxc，因此业务里的普通 .ts 工具文件没有后备转换。应依据 code_type 选择 parser 语法。属于局部修复。

### C16：.ts 文件没有进行国际化转换

位置：[lib.rs](../../src/lib.rs#L124)、[ts/vite-plugin.ts](../../ts/vite-plugin.ts#L76)。

```ts
import { t } from 'jinge';
export const label = t('hello');
```

intl=2 时仍输出 `t('hello')`，没有转换成 `t('m3HSJL')`。插件特意在启用国际化时处理 .ts，但 Rust 只在 code_type=2 时执行包含国际化逻辑的 visitor；原先独立的 IntlTransformVisitor 调用被注释。

应把普通国际化调用转换与 TSX 模板转换分开，确保 .ts 也生效，避免为此把 .ts 中的普通函数误判成组件。

### C17：插槽条件分支丢失用户指定的 host 参数

位置：[parser/slot.rs](../../src/parser/slot.rs#L482)、[parser/mod.rs](../../src/parser/mod.rs#L111)。

```tsx
export function A(props, hostCustom) {
  return <>{props.children ? <p>yes</p> : <p>no</p>}</>;
}
```

分支使用不存在的 `rthost$jg$`，实测 ReferenceError。临时 context 继承了 parent 类型，却没有继承 host_ident。应验证条件插槽的 `?:`、`&&`、`||`、`??`，以及嵌套插槽的 host。

### C18：DOM 属性中的 t() 参数不会随响应式值更新

位置：[parser/expr.rs](../../src/parser/expr.rs#L230)，与运行时 IntlWatcher 接口共同构成问题。

```tsx
return <div title={t('Hello {name}', { name: state.name })} />;
```

字典已提供格式化函数时，首屏 title 是 Hello Alice；修改 name 为 Bob，title 仍是 Hello Alice。`IntlWatcher` 只监听语言切换，传入的是当时求值的普通 params 对象。相同 t() 直接用于文本 children 的专门编译路径能正确更新，是正常对照组。

应统一两条编译路径对参数依赖的处理，不要只补一个属性特例。

### C19：只有富文本组件参数时，默认文本传错参数位

位置：[parser/intl.rs](../../src/parser/intl.rs#L190)、[parser/tpl.rs](../../src/parser/tpl.rs#L101)。

```tsx
return <div>{t('Hello <B>world</B>', { B: (text) => <b>{text}</b> })}</div>;
```

intl=1 且没有对应字典时，实际显示 hash `NH9/Bc`。组件参数被提取后，运行时 params 为空，但默认文本直接作为第 4 个参数传给 `renderIntlRichText`，占据 params 的位置；defaultText 实际应是第 5 个参数。

这里只要求保留默认文本回退，不主张运行时应把默认文案直接渲染成富文本。无 params 时补 undefined 占位即可修正参数位。

### C20：国际化产物直接以原始 key 构造函数名

位置：[ts/intl/compile.ts](../../ts/intl/compile.ts#L74)。

默认 hash 用 Base64，可能包含 `/` 和 `+`。真实文案 `Hello <B>world</B>` 的 hash 是 `NH9/Bc`。富文本编译会生成 `function T_NH9/Bc_B(...)` 等非法标识符。自定义 `greeting-title` 也一样。

应将字典 key 与内部函数标识符分开，后者使用安全、唯一的编号。直接改 hash 算法会影响现有翻译 key，不应为了修函数名而改字典协议。

### C21：翻译文本直接拼接模板字符串，未转义反引号和反斜杠

位置：[ts/intl/compile.ts](../../ts/intl/compile.ts#L115)。

翻译 `Hello \` {name}`生成不闭合的模板字符串，编译失败。静态文本应作为 AST 字符串/模板片段生成或充分转义，与`{name}` 占位符分开处理。应验证反引号、反斜杠、换行和占位符组合。

### C22：CSV 写出遗漏换行字段的引号

位置：[ts/intl/helper.ts](../../ts/intl/helper.ts#L43)。

`writeCsv` 只在字段包含逗号或双引号时加引号，字段仅含 `\n`/`\r` 时裸写。已复现写出 `hello\nworld` 后，`parseCsv` 抛 `Invalid Record Length`。提取阶段生成的翻译表可能因此损坏。可局部修复，优先加读写往返测试。

## 需要明确语法约束或改进诊断的地方

这些不必全部扩展为新能力，但应保证用户能在编译阶段得到确定的反馈。

- **组件提前 return、return 后的空语句**：[visitor.rs](../../src/visitor.rs#L43) 只转换函数体最后一个语句恰好是 return 的情况。`if (props.ok) return <p>yes</p>; return <p>no</p>` 留下前一个 JSX，`return <p>yes</p>; ;` 甚至整段不转换。最终 JS 模块报 `Unexpected token '<'`。若框架坚持单个最终 return，应该明确拒绝其他 JSX return，并允许忽略尾部空语句。
- **map 解构参数**：[map.rs](../../src/parser/map.rs#L286) 的 `pat_to_atom` 返回 None 后直接覆盖参数，`map(({name}) => <p>{name}</p>)` 变成引用未定义 name。可以继续不支持解构，但必须在丢弃参数前报错，不能只发缺少 key 的警告。第三个 map 参数、默认参数也应建立负例。
- **诊断不影响 transform 成功状态**：[common.rs](../../src/common.rs#L175) 的 emit_error 只输出 stderr，调用方仍得到成功结果。`<div {...{id:'x'}}>` 已验证“报错但返回丢属性的产物”。应收集诊断并返回失败，让 Vite 正常展示带位置的错误。
- **parse 错误与未实现分支走 panic**：[lib.rs](../../src/lib.rs#L109) 使用 expect，[visitor.rs](../../src/visitor.rs#L213) 对对象插槽的 optional chain 使用 todo!。Neon 在当前构建下把 panic 转为 JS Error，实测没有让 Node 进程直接退出，但错误为 internal error，附带 Rust panic 日志，不能当作稳定诊断接口。应返回普通编译错误，并做“失败后继续编译下一文件”的测试。
- **国际化识别只比较名称 t**：[parser/intl.rs](../../src/parser/intl.rs#L97) 已明确 TODO：同名本地函数可能被误改，import alias 不识别。后续应依据绑定身份识别；这是当前已知限制，不计作本轮新发现。

## 其他已确认的接口与维护问题

- binding 的 sourcemap 参数被忽略：[lib.rs](../../src/lib.rs#L158) 写死 true。所有传 false 的探针仍返回非空 map；Vite 包装层也把默认值设为 true。主要是接口与编译开销问题，优先级低于误编译。
- 国际化 key 的 trim 规则不一致：Rust 保留 options.key，TS 提取工具会 trim，空字符串处理也不同。属于静态审查确认的协议差异，应补跨语言对照测试后统一规则。
- 国际化 CLI catch 只打印错误，没有设置失败退出码。正式测试应对子进程的退出码断言，避免流水线把失败当作成功；本轮未单独对子进程复现退出码。
- 国际化富文本提取用正则替换函数参数名：[ts/intl/helper.ts](../../ts/intl/helper.ts#L175)。存在误改字符串、对象属性名和嵌套遮蔽的风险，参数名包含 `$` 时正则语义也不同。需要 AST 改写；此项为风险判断，本轮未单独制作行为复现。
- 块体 map 的 key、命名空间、IDL 属性等例子说明 lint/类型检查通过不足以证明产物可执行。没有把当前测试覆盖率当成正确性的证据。

## 建议的测试体系

建议按“Rust 转换测试 + JS 运行时行为测试 + TS 工具/插件测试 + 少量真实浏览器测试”建立四层。无需先重构成多个 crate。

### 1. Rust：转换、结构断言与诊断 fixture

当前 cargo test 已能运行 Neon crate 内部单元测试，先在 lib.rs 增加 `#[cfg(test)] mod tests;`，让测试直接调用编译核心即可，不必为测试引入跨进程 Node binding，也不必先添加 rlib 或拆分整个仓库。

先把固定 assert!(false) 的调试测试替换为真实断言。每个 fixture 建议包含：

```text
input.tsx / input.ts
options.json          # 文件类型、intl 模式、sourcemap 等
output.js             # 可读快照
components.json       # HMR 组件名
errors.json           # 有意不支持的输入与位置
```

快照用于审阅生成策略变化，但还要有结构断言：输出可按纯 JS 重新解析、没有残留 JSX、import/组件元数据正确。对 C08 那种非法赋值，还需 JS 引擎真正加载，不能只依赖一个宽松解析器。

优先 fixture：文本/Unicode、DOM/IDL/SVG、简单路径与动态路径、运算符、函数调用/可选链、map 及 key、条件分支、命名/默认/透传插槽、事件与 capture、国际化所有模式、标识符冲突/作用域遮蔽、.ts 专有语法。

错误快照应只断言稳定的诊断代码、消息和位置，避免固定颜色、绝对路径和 Rust panic 栈。合法输入不能 panic；不支持的输入也不能返回看似成功但已丢语义的产物。

### 2. JS：真实编译产物与真实运行时联测

这层应使用**从当前源码构建的本地 .node**，明确禁止自动退回 npm 平台包。否则 Rust 修改后很容易仍在测试上一个发布版本。

推荐测试文件使用 TypeScript，runner 可沿用 Jest + jsdom，与 jinge 现有测试工具一致。Rust 单元测试负责大部分纯转换覆盖，JS 集成测试集中覆盖语义，避免所有 fixture 都启动一次浏览器。

每个行为用例的流程：

1. 编译 TSX，使用 ESM 加载产物，将 jinge import 映射到测试依赖的运行时。
2. 挂载并断言真实 DOM 的文本、property、attribute、namespace。
3. 修改 vm 数据，等通知/表达式 watcher 队列稳定，再断言 DOM。不要只等一层微任务就假设所有表达式已刷新。
4. 验证 key 对应的 DOM/组件身份，而不只比较最终文本。
5. 卸载组件，再变更数据，断言没有继续更新与残留监听。

必须保留对照：纯标识符按现有契约不生成 watch；普通对象是一次性取值；PathWatcher、DymPathWatcher、ExprWatcher 都有成功用例。此前 For 直接数组的运行时修复，应加入从 `.map()` 源码开始的联测，避免只测手工构造的 props。

编译器仓库正式引入固定版本的 jinge 开发依赖即可起步；本地联合开发可通过 workspace/link 切换源码，但 CI 必须明确使用哪个运行时版本。另保留与当前发布版的兼容性 smoke test，避免双方仅在本地一起改了接口却误以为兼容。

### 3. TS 工具与 Vite 插件

- 国际化：Rust/TS hash 及自定义 key 对照；提取→CSV→编译→重新解析产物→格式化/渲染的完整往返。覆盖中文、emoji、多行、逗号、引号、反斜杠、反引号、Base64 特殊字符、自定义 key、富文本与参数更新。
- CLI：参数缺失、输入不存在、CSV 损坏时断言 stderr 和非零退出码；工具函数不应直接 process.exit，退出留给 CLI 入口。
- Vite：分别执行 build/serve hook，验证 .ts/.tsx 分流、query id、intl=0/1/2、alias、sourcemap、HMR 组件注册。再做一个小型 fixture app 的 `vite build` 与 dev transform 集成测试，不只 mock 所有 hook。
- HMR：编译器返回的 parsedComponents 与实际模块作用域一致；默认导出、匿名函数、嵌套函数、重复命名均需建立约束用例。不能把没有元数据的匿名函数默认判为已支持 HMR。

### 4. 浏览器：少量必须覆盖环境行为的用例

建议只保留实际点击更新、SVG 显示/namespace、输入控件 property、key 重排、HMR 与错误 overlay 等小集合。多数转换和通知语义在前两层验证，浏览器测试负责真实环境差异。

当前 workspace 的 jsdom 探针已经证明该路径可行，但本轮没有完成真实 Vite HMR/浏览器端到端测试，不能把它们列为已验证。

### 落地顺序与修复范围

1. 先搭最小 Rust fixture runner、JS 编译产物联测，迁移本轮复现；已知问题先登记为明确的 expected failure，保留失败原因，不能用大范围 skip 隐藏。正常契约先成为 CI 门禁。
2. 局部修复优先：Unicode、.ts parser 模式、SVG 继承、属性名赋值、独立 slot import、key 路径、Intl 参数占位、安全函数名、模板片段转义、CSV 引号。常量 DOM 属性需与运行时共同确认统一策略。
3. 单独讨论：方法调用与回调依赖语义、标识符 hygiene/绑定身份、spread 与 slots 的合并、是否允许多个 JSX return。这里有局部问题，但不能靠一两条字符串替换完成可靠支持。
4. 整理 transform 的 Result/诊断接口，让负例成为可测试契约；即使未来 TS 层增加静态约束，Rust 仍需在入口防御不支持的 AST，避免 Vite 未做类型检查时静默误编译。
5. 普通 PR 跑 lint、cargo test、JS/TS 测试和一次 fixture build；浏览器 smoke 单独执行。发布矩阵对每个平台的 .node 至少执行一次加载/简单转换 smoke，再发布。

快照更新必须人工审阅，不能把误编译的新输出直接批量接受。覆盖率可以之后接入，但先确保测试能发现语义变化，而不是只追求覆盖数字。

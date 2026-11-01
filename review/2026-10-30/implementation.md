# 本轮修复与测试结果

2026-10-31。决策依据见 decisions.md；测试入口和覆盖率口径见 ../../test/README.md。

## 已完成

- C01：当前起始源码已有 Unicode 修复，本轮补充实际编译渲染回归保护。
- C04：修复普通可选函数调用被误判为简单成员路径、丢失调用的问题，覆盖空函数短路和数据更新；含副作用的可选链依赖提取另列待讨论。
- C05：表达式生成参数改用 `$jg$aN` 前缀，按已确认方案处理。
- C06：使用具体源节点 Span 标记国际化调用，避免同 SyntaxContext 的普通函数调用被误替换。
- C07：修复静态 SVG 后代继承，处理 foreignObject 的 HTML 边界及重新进入 svg。跨 If/For、插槽和独立组件的传播另列待讨论。
- C08、C09：组件属性使用安全的 computed 赋值；常量 HTML IDL 属性采用 property 赋值，与现有响应式规则一致。未修改通用运行时 setAttribute 接口。
- C10：拒绝 DOM spread、组件 spread 与新增 props/children/命名 slot 混用；保留组件原对象透传和独立特殊属性的现有路径。
- C11：单独对象字面量插槽转换后正确注入运行时 imports。
- C12：增加每次 transform 的词法符号上下文及 scope 入栈出栈，保留 SWC resolver 的绑定 Id，map 替换不再按名称判断；移除已不需要的手写参数遮蔽判断。增加当前文件已知非响应式来源的模板 member/map 告警；import、未知函数返回值和 context 等不提示。覆盖局部块、解构、catch 和 shorthand。
- C13：普通标识符索引仍生成 state 路径监听；let/var 等非 const key 绑定提示 key 自身变化不响应。const 解构 key、动态 vm key 保留相应行为。
- C14：修复混合路径和字符串转义，提取块体最终 return、fragment 等已有结构的 key；回调局部计算变量不能安全外提时明确拒绝。
- C15、C16：按类型选择 TS/TSX parser；普通 .ts 国际化使用独立 visitor。
- C17：条件插槽临时上下文继承自定义 host，覆盖条件及逻辑表达式。
- C18：组合参数依赖 watcher 与 locale watcher，使属性/复合表达式内的 t() 随参数及语言更新；使用既有运行时接口，避免重复求值参数表达式。
- C19：仅富文本组件参数时补充 undefined 占位，恢复默认文案参数位置。
- C20：内部函数使用独立编号，支持默认 hash 和任意自定义字典 key；无需修改 hash 协议。
- C21、C22：翻译文本安全转义；CSV 含 CR/LF 字段正确加引号。
- 其他：编译错误返回普通 JS Error，包含位置，解析失败后仍可继续编译；残留 JSX、不支持的 map 参数等明确报错；尾部空语句可编译；HMR 警告不阻断合法匿名组件；binding/生产插件遵守 sourcemap；Rust/TS key trim 一致；CLI catch 设置失败退出码；富文本函数参数替换使用 TS AST 和绑定身份，文件级 checker 以 WeakMap 缓存。

首批 44 个用例在实现修改前 38 个失败、6 个通过，修复后全部通过。扩展后主 JS/TS 测试 148 个全部通过，另有 4 个 Rust 测试。51 份目标 JS 快照同时有真实执行验证。运行时源码未修改；为联测重新构建了同级运行时的忽略产物。

## 跳过并保留讨论

1. C02、C03：按事先决定不改方法接收者/方法体及嵌套回调的依赖语义。
2. SVG 跨边界：2026-11-01 已按标签名静态识别 SVG 专有标签，修复 circle/path 等跨组件、插槽、If/For 的场景；a/title/style/script 等同名标签仍沿用原上下文规则，跨独立组件的歧义场景继续保留讨论。
3. `map(item => { const id = item.id; return <p key={id}/> })`：本轮明确报错，并建议完整书写原始 key 表达式，例如 `key={item.id}`。支持它需要分析并外提局部计算，避免把闭包局部变量放到 For 外部。
4. `return <>{props.children && <b/>}</>` 在没有 slot 时返回空数组，运行时要求函数组件至少有一个节点。如何统一空模板的 comment/host 占位需进一步讨论；未修改运行时根节点契约。
5. `state.obj?.[key() + state.n]`：提取 watcher 可能提前执行 key()，违反短路副作用语义。统一处理惰性依赖需要更大表达式设计，本轮只修普通可选函数调用，不声称完整修复所有 optional chain。
6. 国际化 t 的 import alias/同名函数识别仍是原 review 已注明的历史限制，未在本轮扩大到统一国际化绑定识别设计。
7. CSS 动画及真实浏览器 HMR/overlay 未为覆盖数字强行补齐；当前覆盖率和环境边界见测试 README。

尚未修复的 5 个可执行失败复现位于 test/integration/pending.test.mjs，单独命令运行，未混入主测试成功数字。未来 TS 返回类型推断、仅支持 TS/TSX、告警抑制注释、spread 组合能力及此前拒绝语法的扩展仍保留在 decisions.md。

### 第 2 点示例：SVG 穿过组件、条件、列表和插槽（修复前复现）

下面可以直接作为示例项目的 `App.tsx`，入口沿用项目现有的 `bootstrap(App, ...)`。预期五个区域都能显示圆形，条件区域可通过按钮显示/隐藏；修复前只有直接嵌套的绿色圆形正确。2026-11-01 修复后，这五种写法都正确使用 SVG namespace。

```tsx
import { vm, type JNode, type WithChildren } from 'jinge';

function Dot() {
  return <circle id="component-dot" cx="30" cy="30" r="20" fill="tomato" />;
}

function SvgFrame(props: WithChildren<JNode>) {
  return (
    <svg width="120" height="60">
      {props.children}
    </svg>
  );
}

export function App() {
  const state = vm({ show: true, dots: [{ id: 'one', x: 30 }] });
  return (
    <div>
      <p>直接嵌套：应显示圆形</p>
      <svg width="120" height="60">
        <g>
          <circle id="direct-dot" cx="30" cy="30" r="20" fill="green" />
        </g>
      </svg>

      <p>独立组件：修复前圆形不显示</p>
      <svg width="120" height="60">
        <Dot />
      </svg>

      <p>条件分支：修复前圆形不显示</p>
      <button on:click={() => (state.show = !state.show)}>切换</button>
      <svg width="120" height="60">
        {state.show && <circle id="if-dot" cx="30" cy="30" r="20" fill="tomato" />}
      </svg>

      <p>列表：修复前圆形不显示</p>
      <svg width="120" height="60">
        {state.dots.map((dot) => (
          <circle key={dot.id} cx={dot.x} cy="30" r="20" fill="tomato" />
        ))}
      </svg>

      <p>插槽：修复前圆形不显示</p>
      <SvgFrame>
        <circle id="slot-dot" cx="30" cy="30" r="20" fill="tomato" />
      </SvgFrame>
    </div>
  );
}
```

问题不在于 `circle` 没插入 DOM，而在于它被创建成了错误 namespace 的元素。用修复前的编译器编译并挂载这个示例，读取各个 circle 的 `namespaceURI`，结果为：

| 区域                                   | 应有 namespace | 修复前 namespace |
| -------------------------------------- | -------------- | ---------------- |
| 直接 `<svg><g><circle /></g></svg>`    | SVG            | SVG              |
| `<svg><Dot /></svg>`                   | SVG            | HTML             |
| SVG 内的条件分支                       | SVG            | HTML             |
| SVG 内的 map 列表                      | SVG            | HTML             |
| `<SvgFrame><circle /></SvgFrame>` 插槽 | SVG            | HTML             |

这里 SVG 指 `http://www.w3.org/2000/svg`，HTML 指 `http://www.w3.org/1999/xhtml`。检查元素时即使看到 `<svg>` 下面有 `<circle>` 标签，也不代表它是可绘制的 SVGCircleElement。

本次采用局部静态规则：对运行时 JSX 类型中已有的 SVG 专有标签直接使用 SVG namespace，因此无需知道组件调用位置，也无需新增运行时传播协议。`foreignObject` 自身为 SVG，子节点切回 HTML；其中嵌套 svg 再进入 SVG。a/title/style/script 等同名标签仍依赖已有上下文，在独立组件边界后的歧义场景暂不扩展。

新增 4 个 Rust 回归测试（遍历全部 59 个 SVG 专有标签并检查边界）、3 份完整目标代码快照、5 个实际执行集成测试和 1 个 Chrome 几何渲染测试。修复前新增 Rust 集 3 失败/1 通过、集成集 4 失败/1 通过、Chrome 1 失败；修复后全部通过。两个原 SVG TODO 已被常规回归覆盖替代。

## 后续：按 Rust 优先重组测试

根据用户澄清的重点，原 JS/TS 验证代码已集中迁移到 `test/integration`；新增 `test/rust` 和 `test/e2e`。此节取代上文初轮“仅 4 个 Rust 测试、没有真实浏览器验证”的现状描述，初轮结果作为历史证据保留。

- Rust 当前 105 个直接测试：95 组独立 fixture，含 73 份完整目标 JS 和 22 份完整诊断；另有纯 JS 重解析、组件元数据、sourcemap、hash、失败恢复和并发隔离检查。运行 `cargo test --locked` 不需要 Node 或运行时。
- 最初 51 份目标代码原样迁至 Rust fixture 目录，JS 联测继续读取这些输入和预期，没有重复维护两套快照；原 148 个 JS/TS 用例迁移后全部通过。
- 本机 Chrome `156.0.8078.73` 可通过 Playwright 使用。10 个真实浏览器用例完整运行两轮，20/20 通过；包括 transition/keyframes、动画取消和销毁、TransitionGroup、WebSocket HMR、错误 overlay 与修正恢复。
- 浏览器动画采样曾在初轮漏过首次动画的中间值；加入初始帧提交和 10ms 有界采样后，相关用例独立连续 5 次通过，之后完整两轮通过。
- overlay 测试新增断言复现了一个小问题：模块编译失败时 HMR accept 回调可能收到 undefined，原插件访问 `__hmrUpdate__` 产生第二次异常。已局部改为 `newModule?.__hmrUpdate__()`，新断言通过，正常更新流程不变。
- 覆盖率入口现在同时执行 Rust 测试二进制和 native binding 联测；不把这两层和浏览器运行结果混成同一覆盖率数字。CI 已加入 Chrome 安装、e2e 和报告上传，远端尚未执行。

具体目录、命令及统计口径见 test/README.md 和各子目录 README。

## 后续：局部变量丢失响应的诊断（2026-11-01）

key 的报错现在明确建议完整书写原始表达式，例如 `key={item.id}`；回调参数名为 row 时，例子相应变成 `key={row.id}`。实际属性路径仍应按原表达式填写。

普通文本和属性则给告警，不阻断编译。例如：

```tsx
items.map((item) => {
  const name = item.name;
  return (
    <p key={item.id} title={name}>
      {name}
    </p>
  );
});
```

使用 name 的位置提示：局部变量保存了响应式数据的读取结果，会失去对原始路径的响应式追踪；请在 JSX 中完整书写原始表达式，不要先提取为局部变量。建议改成：

```tsx
items.map((item) => (
  <p key={item.id} title={item.name}>
    {item.name}
  </p>
));
```

告警覆盖 map 数据/index、已知 vm 数据的成员读取，以及通过这些结果派生的局部变量、解构和简单后续赋值；适用于文本、DOM 属性和组件属性。对象局部别名即使仍能响应其自身成员变更，也可能失去对原始对象替换的追踪，因此同样提醒。此改动只诊断，不把局部变量转换成派生响应式状态。

普通常量、直接成员路径、未知外部来源、同名遮蔽和延迟执行的函数闭包不触发这条新告警。不进行跨函数返回值或控制流的完整推断；C02/C03 仍保留讨论。

先运行新增用例确认失败：首批集成集 10 失败/5 通过，Rust key 提示回归 1 失败。修复后扩展诊断集共 22 个通过；全量 Rust 121、集成 189、Chrome 13 个通过。待讨论 TODO 仍为 5 个，不计入主集。完整目标代码快照无需修改，仅更新了 key 错误诊断快照。

## 后续：函数组件和可变普通变量告警（2026-11-01）

范围扩展到当前 TSX/JSX 文件内的声明，不限于 map。已知 VM 或函数组件 props 的成员被提前读取、解构或用于派生计算，再作为 JSX 文本、DOM 属性或组件属性使用时告警。例如：

```tsx
export function App(props) {
  const name = props.name;
  return <span>{name}</span>; // 建议直接完整书写 props.name
}
```

重新赋值则单独提示“重新赋值不会自动更新模板或重新绑定监听”，建议使用 VM 成员保存和修改状态：

```tsx
export function App() {
  let count = 0;
  return <button on:click={() => count++}>{count}</button>;
  // 建议改成 const state = vm({ count: 0 })，模板和回调都使用 state.count
}
```

使用已有 resolver 的绑定身份，在来源分析前收集整文件的写入；支持直接/复合赋值、++/--、对象和数组解构赋值，以及 for-in/for-of 对已有变量的赋值。回调先定义、变量后声明也能检测。此检测没有引入控制流图或跨函数返回值推断，也没有退回到所有 let 一律告警。普通变量派生的 const（如 const label = count + 1）同样可能截断原始依赖，因此也会提示。

正常的 VM 成员修改不算重新绑定 VM。`let state = vm(...)` 但没有 state = ... 时不触发重赋值告警；state = vm(...) 后，使用 state 的模板会提示。只在事件闭包内读取/修改的变量不触发模板告警；同名遮蔽按绑定区分。外部导入和未知工厂返回值仍不猜测；普通 helper 的参数不被当成组件 props，只有按现有规则被识别为模板函数的首参数才标记为响应式来源。

先新增 34 个用例验证：20 个失败、14 个通过；修复后扩展至 36 个，包括对象 shorthand 派生和 count = count + 1。目标代码保持原生成策略；这里只增加诊断，不自动修复响应式行为。

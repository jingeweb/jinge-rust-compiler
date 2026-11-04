# JS/TS 编译产物联测

原有 JS/TS 验证代码已集中到本目录，265 个主用例和 3 个单独运行的待讨论复现；原 C02 已转入正式方法调用回归。

```sh
pnpm test:integration
pnpm test:pending
```

先从当前 Rust 源码构建 native binding，再验证真实生成代码、DOM 更新和运行时清理。没有回退到 npm 平台 binding。运行时默认位于同级 `../jinge`，需先 `pnpm build`；可通过 `JINGE_RUNTIME_ROOT` 指定其他路径。

- `intl-bindings.test.mjs`：直接导入别名、同名/遮蔽、语言与参数更新、TS 提取到富文本字典渲染的一致性。
- `slot-empty.test.mjs`：静态插槽条件的 comment 占位、根节点归属、销毁、替换、If 切换和 keyed 列表移动。
- `component-reactivity.test.mjs`：函数组件/文件级 VM 和 props 提前取值、普通变量重新赋值、解构赋值、增减、循环赋值、VM 重新绑定及不告警对照。
- `local-snapshots.test.mjs`：局部变量截断响应式路径的告警、key 完整书写建议，以及未知来源/常量/函数闭包/遮蔽对照。
- `svg.test.mjs`：SVG 专有标签跨组件、If/For、默认/命名插槽、foreignObject 与 HTML 对照。
- `method-calls.test.mjs`：接收者/方法替换、原生/动态/可选调用、显式参数依赖、不深度订阅及原 C02。
- `dom-paths.test.mjs`：多选初始化顺序、字符串长度/索引、动态路径、空值恢复与属性更新。
- `regressions.test.mjs`：修复回归及能力保留对照。
- `modules.test.mjs`、`fixtures.mjs`：从 Rust fixture 目录读取 51 类输入/预期代码，实际挂载、修改数据并断言结果。
- `runtime.test.mjs`：列表、事件、销毁、context、ref、组件、国际化和 HMR。
- `diagnostics.test.mjs`：失败返回、位置、告警和词法边界。
- `tools.ts`：国际化工具、CLI、CSV 和富文本完整链路，通过测试 bundle 的子进程执行。
- `vite.test.mjs`、`vite-app/`：Vite dev/build 的 Node 层联测。
- `pending.test.mjs`：待讨论行为的可执行失败断言，使用 TODO 标记；不计入主集通过数。
- `baseline.json`：首次修复前 44 个用例的历史结果，6 通过、38 失败。保留当时的命令路径作为历史证据。

常规编译结果比较现在由 Rust 层承担；本层主要提供 JS 引擎与真实运行时的语义验证。

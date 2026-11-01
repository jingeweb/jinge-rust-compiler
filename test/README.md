# 编译器测试

测试分成三层，常规编译结果比较以 Rust 层为主：

| 目录                                 | 目的                                                       | 当前规模                                                 |
| ------------------------------------ | ---------------------------------------------------------- | -------------------------------------------------------- |
| [rust](rust/README.md)               | 直接调用 Rust 编译核心，比较完整目标代码、诊断和组件元数据 | 136 个测试；108 组 fixture，其中 86 份目标 JS、22 份诊断 |
| [integration](integration/README.md) | 原有 JS/TS 验证集中存放，实际加载编译产物并联测运行时      | 254 个主用例；4 个待讨论复现单独运行                     |
| [e2e](e2e/README.md)                 | 真实 Chrome + Vite，检查 CSS 动画、列表、HMR 和错误恢复    | 15 个用例，全量通过                                      |

## 运行

```sh
pnpm test:types        # 正常测试脚本和示例的 TypeScript 检查
pnpm test:rust         # cargo test --locked；无需 Node 或运行时
pnpm test:integration  # 当前源码 binding + JS/TS 联测
pnpm test             # 先 Rust，再 integration
pnpm test:chrome      # 检查本机 Chrome
pnpm test:e2e         # 启动真实浏览器
pnpm lint
```

Rust 单层只需 Rust stable。其他层需要 Node 24+、pnpm；先运行 `pnpm install --frozen-lockfile`。联测运行时默认使用同级 `../jinge`，也可以设置 `JINGE_RUNTIME_ROOT=/absolute/path/to/jinge`；运行时需先安装依赖并执行 `pnpm build`，避免 dist 落后于源码。

CI 当前固定检出的运行时为 jinge 4.1.9，提交 `daf0d92119bc123fdae7db46cb787c2e369dc7c9`；CI 显式检出该提交，不依赖尚未发布的 npm 版本。该提交包含 D02 修复，本次已同步 CI ref。CI 也安装 Chrome 并执行 e2e，但本机通过不代表远端 CI 已运行。

## 编辑器和类型检查

`test/tsconfig.json` 为正常测试脚本及浏览器示例提供 DOM、JSX 和同级 jinge 的类型配置；独立运行 `pnpm test:types`，不改变生产代码的 tsconfig。JS 测试纳入项目解析，但不开启 checkJs。

Rust 原始输入使用 `input.tsx.txt` 数据文件，其中包含故意不合法的语法及按普通 TS 解析的输入。TypeScript 的 exclude 不能阻止编辑器为单独打开的 TSX 建立推断项目，因此用数据扩展名避免编辑器误报，而不在原始输入中插入抑制注释或关闭工作区检查。Rust 和 JS 联测读取的内容保持原样，编译文件类型仍由 options.txt 控制。

## 编译结果与快照

输入和目标代码统一放在 `rust/fixtures/<name>/`，Rust 用例直接读取文件。最初 51 份已验证目标代码原样迁移，没有重新接受快照；JS 行为测试继续复用这些输入和预期。新增 fixture 进一步覆盖本轮修复回归和拒绝语法。

```sh
cargo test --locked tests::codegen::case_reg_computed_const
pnpm test:update
```

默认测试只读比较；更新命令只重写成功用例的目标 JS，必须审阅 diff。诊断和组件元数据不自动接受。成功代码还会重新按纯 JS 解析，JS 联测则验证真实执行语义。

历史修复前记录保存在 `integration/baseline.json`：首批 44 个用例为 6 通过、38 失败，修复后全部通过。后续扩展主集不使用 skip/TODO 隐藏失败。

## 待讨论用例

```sh
pnpm test:pending
```

4 个复现使用明确的 TODO 标记，命令成功不表示问题已修复；包括 C02/C03、局部计算 key 外提及带副作用的可选链。它们不计入主集通过数。

本次完整验证记录见 [verification.json](verification.json)。Rust 136、integration 254、e2e 15 个用例均通过。修复前失败与修复后验证见 [D01/D02 记录](../review/2026-10-30/demos-fixes.md)。

## 覆盖率与浏览器报告

```sh
rustup component add llvm-tools-preview
pnpm test:coverage
```

覆盖率命令先构建并执行插桩的 **Rust 测试二进制**，再构建插桩 native binding 执行 JS 联测，最后合并 LLVM profile。无需重新插桩全部依赖；结束时恢复普通 debug binding。Rust 统计排除测试目录、依赖和标准库；原有文本单元测试也已移至 Rust 测试目录。

- `coverage/rust-summary.txt`、`coverage/rust-html/index.html`：Rust line/region/function 覆盖；稳定 LLVM 当前未提供 Rust 分支数据，不把 region 当作 branch。
- `coverage/compiler-ts/index.html`：TS 原始源码覆盖，包含未加载模块的零覆盖基线。
- `coverage/runtime/index.html`：本仓库 Node 联测对 `jinge/src` 的覆盖，没有合入运行时自身测试或浏览器 e2e 覆盖。
- `coverage/e2e-report/index.html`、`coverage/e2e-results.json`：真实 Chrome 结果，覆盖率命令不会删除它们。浏览器失败时还保留截图和 trace。
- JS/TS 使用 V8 + source map，存在行级映射局限；不与 Rust region 指标混算。`report-v8.mjs` 可以在已有原始记录上重新生成 JS/TS 报告。

上轮（D01/D02 修复前）行覆盖率，未在本次重新测量：Rust **86.95%**，编译器 TS **91.35%**，Node 联测对运行时的覆盖 **81.48%**。这些数值不包含浏览器覆盖率。

CSS 测试检查真实中间样式、CSSAnimation 及浏览器可信结束事件，不手动派发结束事件。浏览器也实际修改测试专用 TSX，验证 WebSocket HMR、overlay 和恢复。各平台 native binding 发布矩阵及更多浏览器仍未覆盖。

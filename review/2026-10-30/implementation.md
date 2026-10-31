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
2. SVG 跨独立组件、插槽及生成的 If/For：当前 Parent 混合了组件所有权和 DOM namespace。完整修复需要解耦上下文，并为独立组件定义传播协议，超出局部静态继承修复。
3. `map(item => { const id = item.id; return <p key={id}/> })`：本轮明确报错。支持它需要分析并外提局部计算，避免把闭包局部变量放到 For 外部。
4. `return <>{props.children && <b/>}</>` 在没有 slot 时返回空数组，运行时要求函数组件至少有一个节点。如何统一空模板的 comment/host 占位需进一步讨论；未修改运行时根节点契约。
5. `state.obj?.[key() + state.n]`：提取 watcher 可能提前执行 key()，违反短路副作用语义。统一处理惰性依赖需要更大表达式设计，本轮只修普通可选函数调用，不声称完整修复所有 optional chain。
6. 国际化 t 的 import alias/同名函数识别仍是原 review 已注明的历史限制，未在本轮扩大到统一国际化绑定识别设计。
7. CSS 动画及真实浏览器 HMR/overlay 未为覆盖数字强行补齐；当前覆盖率和环境边界见测试 README。

以上可执行失败复现位于 test/pending.test.mjs，单独命令运行，未混入主测试成功数字。未来 TS 返回类型推断、仅支持 TS/TSX、告警抑制注释、spread 组合能力及此前拒绝语法的扩展仍保留在 decisions.md。

## 后续：按 Rust 优先重组测试

根据用户澄清的重点，原 JS/TS 验证代码已集中迁移到 `test/integration`；新增 `test/rust` 和 `test/e2e`。此节取代上文初轮“仅 4 个 Rust 测试、没有真实浏览器验证”的现状描述，初轮结果作为历史证据保留。

- Rust 当前 105 个直接测试：95 组独立 fixture，含 73 份完整目标 JS 和 22 份完整诊断；另有纯 JS 重解析、组件元数据、sourcemap、hash、失败恢复和并发隔离检查。运行 `cargo test --locked` 不需要 Node 或运行时。
- 最初 51 份目标代码原样迁至 Rust fixture 目录，JS 联测继续读取这些输入和预期，没有重复维护两套快照；原 148 个 JS/TS 用例迁移后全部通过。
- 本机 Chrome `156.0.8078.73` 可通过 Playwright 使用。10 个真实浏览器用例完整运行两轮，20/20 通过；包括 transition/keyframes、动画取消和销毁、TransitionGroup、WebSocket HMR、错误 overlay 与修正恢复。
- 浏览器动画采样曾在初轮漏过首次动画的中间值；加入初始帧提交和 10ms 有界采样后，相关用例独立连续 5 次通过，之后完整两轮通过。
- overlay 测试新增断言复现了一个小问题：模块编译失败时 HMR accept 回调可能收到 undefined，原插件访问 `__hmrUpdate__` 产生第二次异常。已局部改为 `newModule?.__hmrUpdate__()`，新断言通过，正常更新流程不变。
- 覆盖率入口现在同时执行 Rust 测试二进制和 native binding 联测；不把这两层和浏览器运行结果混成同一覆盖率数字。CI 已加入 Chrome 安装、e2e 和报告上传，远端尚未执行。

具体目录、命令及统计口径见 test/README.md 和各子目录 README。

# 成员调用的 this 修复与四仓库验收

当前 working tree 在 Jinge 增加 MethodPathWatcher，compiler 为响应式成员调用生成该 watcher。依赖契约已经明确：接收者与方法各自非 deep 订阅，显式参数路径沿用现有追踪，不自动追踪方法体/闭包依赖。

## 实现范围

- 普通、括号包裹的成员调用，既有动态接收者路径与动态方法名。
- 字符串等原始值接收者；接收者替换但方法仍共享同一引用。
- 方法独立替换，旧订阅清理，方法 getter 初始化不重复读取。
- 可选接收者/可选调用、缺失必需方法的错误、spread 参数及嵌套方法调用。
- 成员函数作为值传递不自动绑定；构造调用不使用此方案。

ExprVisitor 和 MemberExprReplaceVisitor 对方法调用采用一致的专用处理。只有生成代码实际使用 MethodPathWatcher 时才加入 import，避免给其他模板增加新 runtime 导出要求。reg-optional-call 是唯一改变的既有快照，已检查其变更为方法 watcher；其余快照保留。

Call 返回值的成员读取（如 pageSelection().all）、方法内部隐式 computed 依赖、回调局部表达式追踪、可选计算属性副作用短路不在本轮范围。pageSelection 的业务依赖仍需显式传入。

## 测试证据

首批 10 项 JS 集成在旧产物下 9 项失败、1 项不绑定普通函数值的对照通过，修复后全部通过。runtime 新 API 未实现时测试因缺少导出失败，之后 10 项通过。原 pending C02 仅验证初始 this，现移入正式回归；方法体非自动依赖另有正式契约用例。

四仓库使用本地变更产物：

| 仓库           | 全量结果                                                       |
| -------------- | -------------------------------------------------------------- |
| jinge          | 35 suites / 361 tests 通过                                     |
| jinge-compiler | release 6、Rust 140、JS/TS integration 265、Chrome e2e 17 通过 |
| jinge-demos    | 45 项 Chrome e2e 通过，全部示例构建通过                        |
| jinge-antd     | 589 正常行为及 19 原问题回归通过，共 608；组件库/站点构建通过  |

类型、lint、格式检查完成。compiler 余下 3 个历史 TODO 单独运行，不计入通过数；antd 的框架独立探针为 F02 新回归 1 项通过，原 F04/F05/F07/F08/F09/F11/F12/F14 共 8 项仍真实失败。

Demos 首轮验收的 ECharts tooltip 用例没有命中图形：新版 ECharts 网格边距使旧固定坐标位于边界外。新旧 compiler 对该示例生成代码完全一致；测试改为相对实际 canvas、落在首个柱内部的坐标，仍通过真实鼠标悬浮验证 tooltip，未调用图表内部 API。修正后全量 45 项通过。

## 本地产物来源与复现

先构建 ../jinge（保留此前授权的 Temporal VM 排除），再在 compiler 构建 native binding 与 TS 包：

```sh
cd ../jinge
pnpm build
pnpm test --runInBand
cd ../jinge-compiler
pnpm build-ts
node test/run.mjs --prepare-only
node scripts/sync-local-consumers.mjs
pnpm test
pnpm test:e2e
pnpm test:pending
cd ../jinge-demos
DEMOS_TEST_PORT=5201 pnpm test
pnpm build
cd ../jinge-antd
pnpm test:verify
pnpm test:framework
```

sync-local-consumers 显式复制 runtime/compiler 的 dist/types，以及 compiler 的本地 index.node，不使用 link、不改锁文件或版本号。先检查 22 个消费作用域（44 项依赖解析）的版本与 router/runtime 身份，再原子替换文件，避免 pnpm 硬链接原地写。源/目标路径与 SHA-256 在 coverage/local-method-consumers.json，旧文件备份在 coverage/local-method-backup。node_modules 重新安装后需要重新同步；源码导出路径不由此脚本覆盖。

compiler 联测与浏览器测试直接使用相邻 Jinge 的构建。demos/antd 从已安装包实际加载这批同步产物，router 确认使用同一份 Jinge。测试报告在各自既有目录，没有借助远端已发布版本。

没有发布、提交或变更版本号。正式发布应先发布配套 runtime，并把 compiler CI 的 runtime pin 更新到含该改动的提交；当前 pin 的旧 runtime 不含新 helper。本轮是本地全量验证，没有运行远端 CI。

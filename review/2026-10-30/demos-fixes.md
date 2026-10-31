# D01 / D02 修复与验证

本记录接续 `demos-failures.md/json` 的修复前调查。先增加回归测试并确认失败，再修改实现，最后执行全量测试。没有修改示例源码或放宽产品行为断言。

## 修复前新增回归集

| 测试层                          | 通过 | 失败 |
| ------------------------------- | ---: | ---: |
| Rust `dom_paths.rs`             |    2 |    2 |
| Node `dom-paths.test.mjs`       |    4 |   10 |
| Chrome `dom-paths.spec.mjs`     |    0 |    2 |
| 运行时 `primitive-path.test.ts` |   12 |   20 |

通过项为能力保留对照。另补 4 组完整 Rust 目标代码快照，用于固定修复后的生成结果。测试加载器会替换引号中的模块名 `jinge`，新增集成用例的普通字符串数据改为 `hello`，避免误替换测试数据。

## 局部实现

- D01：只对 HTML select 的 multiple 初始化调整顺序。先创建 select，设置 multiple / 注册并立即执行对应 watcher，再渲染添加子节点，最后执行原有 value 等属性设置。不增加运行时 API。其它元素生成逻辑保持原顺序。
- D02：`jinge/src/vm/watch.ts` 的 `getValueByPath` 中，路径中间值只在 null/undefined 时停止；基本类型按 JS 属性规则继续读取。保留根节点对象检查，以及原有空路径返回行为。

第二点覆盖空串、中文、emoji UTF-16 长度、静态/动态字符串索引、越界恢复、数字/布尔/bigint/symbol/函数属性、可空多层路径、同长度替换抑制通知、属性/表达式渲染、销毁后的订阅清理。没有改变 C02 方法接收者或方法内部依赖规则。

## 全量结果

- `cargo test --locked`：113 通过，含 99 组 fixture（77 份代码、22 份诊断）。
- `node test/run.mjs`：162 通过。
- `node test/e2e/run.mjs`：12 通过。
- `jinge` 全量 Jest：33 suites / 341 tests 通过。
- `jinge-demos` 全量 Chrome：28 通过。
- 原有 7 个待讨论 TODO 复现仍单独运行，不计入主集通过数。
- 编译器与运行时类型、lint、format，以及编译器 cargo fmt 检查通过。覆盖率未重新测量。
- demos 的 lint、测试类型检查及本轮变更文件格式检查通过；全仓格式检查仍报已有 `packages/simple-todos/src/app.tsx`，本轮未修改该示例。

## 后续提交依赖

本机使用同级运行时工作区的修复。`.github/workflows/test.yml` 仍固定旧运行时 ref；等运行时修复有实际提交后需更新该 ref，否则新增 D02 联测会在 CI 失败。本轮未创建提交，也未运行远端 CI。

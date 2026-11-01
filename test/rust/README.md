# Rust 层：输入与预期编译结果

```sh
cargo test --locked
cargo test --locked tests::codegen::case_reg_computed_const
cargo test --locked tests::contracts
```

完全不依赖 Node、`.node`、浏览器或同级运行时。通过 `src/lib.rs` 的 `#[cfg(test)]` 引入本目录，直接调用私有编译核心 `inner_transform`；无需为测试拆分 crate 或增加公共 API。

当前 `cargo test` 共 **136 个测试**，其中 108 个逐例 fixture 测试、1 个 fixture 注册完整性检查、6 个编译契约测试、2 个 smoke 测试，4 个 DOM/成员路径回归测试、4 个 SVG 标签/上下文回归测试、3 个空插槽分支回归测试、6 个国际化绑定回归测试，以及编译器原有的 2 个文本单元测试。

每个 fixture 在 `fixtures/<name>/` 下独立保存：

```text
input.tsx.txt        原始输入；options.txt 中 type=1 时按普通 TS 解析
options.txt      type、intl 模式及预期 HMR components
expected.js      86 个成功输入的完整目标 JS
expected.error   22 个拒绝输入的完整诊断，含源码位置
```

测试不仅比较字符串：成功产物还会被重新按纯 JavaScript 解析，检查没有语法残留或可恢复解析错误；组件元数据单独比较，关闭 sourcemap 时必须没有 map。诊断 fixture 必须返回普通编译失败，不能意外成功。

`contracts.rs` 另外检查 sourcemap 的开关和源文件名、出错后继续编译、TS 泛型/断言、跨语言 hash 协议向量，以及并发编译的线程隔离。

51 个基础 fixture 复用前一轮已经过实际执行验证的目标代码，没有重新接受快照。新增的回归和诊断 fixture 覆盖参数碰撞、作用域遮蔽、computed key、spread 限制、国际化、可选调用等。`integration/fixtures.mjs` 的行为测试从这里读取输入；预期代码由 Rust 层统一维护。

确需更改生成策略时：

```sh
pnpm test:update
# 或只更新一项
UPDATE_RUST_SNAPSHOTS=1 cargo test --locked case_reg_computed_const
```

默认运行只读比较，不接受新输出。更新开关仅更新成功用例的 `expected.js`；诊断及组件元数据需显式审阅和修改。新增目录还必须在 `codegen.rs` 中注册，否则完整性检查失败。原始 fixture 不参与自动格式化/lint，避免修改故意不合法的源码或使诊断位置变化。

原始输入刻意使用 `.tsx.txt` 扩展名，让编辑器按数据文件处理。部分样例本来就应当解析失败；只在 tsconfig 中排除仍可能被编辑器作为推断项目检查。测试传给编译器的文件类型和源文件名由 runner/options 控制，不取决于数据文件扩展名。

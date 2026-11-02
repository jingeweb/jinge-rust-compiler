# jinge compiler

> 使用 rust 编写的 [jinge](https://github.com/jingeweb/jinge) 框架的模板编译器

## About

`jinge` 框架，一个小巧的前端界面框架。这个仓库是该框架的模板的编译器，基于 `swc` 开发。

关于该框架，详见：https://github.com/jingeweb/jinge。

## Develop

`todo`

## Todo

- 只支持 typescript，不再支持 javascript。考虑下要不要这样做。
- 对于先 `const name = state.name` 这样声明，再在 tsx 中使用 `<span>{name}</span>` 的情况，当前会报告警。应该是可以将这个转成通过 `vmWatch` 来监听的，只不过模板中的 `{name}` 如何也通知更新是个问题，可以讨论下方案实现。
- 对 style 的处理对齐 react 等的 CSSProperties。
  - 对于 `<div style={{ width: state.width, marginTop: 30 }}>` 这种写法，编译成带 watch 的转换。在编译器中转成 `styel={``width:${state.width};margin-top:30px;``}` 这样的字符串 style。运行时不会去做 marginTop 转 margin-top 的逻辑。由 typescript 提供 CSSProperties 的类型能力。
  - 对于 `<div style={state.style}>` 这种写法，类型上也是要求 state.style 是 CSSProperties 类型，但运行时还是要求是 string，否则报错。state.style 由用户通过 `css()` 这个内置编译器糖函数来处理，编译器会在编译时将 css 直接转成带 watch 的 template string 表达式。
    - 但比如 `const x = css({marginTop: 10}); <div style={x}>; x.marginTop = 20` 这种写法怎么处理，还没想清楚。
- 当模板中使用到组件里定义的变量（比如 x.y 这种表达式，或者直接 x.map 这种数组使用），没有使用 vm 包装时，发出警告。避免用户遗漏 vm 包装后又发现界面不响应数据更新浪费时间排查。
- 插槽也可以 watch 监听，从而支持动态插槽。
- 支持嵌套属性，比如:
  ```tsx
  <Table pagination={{ pageSize: state.pageSize }}>
  ```
  转换为：
  ```ts
  // 嵌套的初始化并 watch 代码
  const attrs$jg$_1 = vm({});
  watch(state.pageSize, (v) => attrs$jg$_1.pageSize = v, {immediate: true});
  // 当前版本已经支持的生成的代码
  const attrs$jg$ = vm({
    pagination: attrs$jg$_1
  });
  ...
  ```

## 发布与版本同步

发布流程与 lightstatic 一致：创建 GitHub Release 或手动运行
`.github/workflows/ci.yml`，先构建、发布五个平台的 native 包，全部成功后再
构建并发布 `jinge-compiler` 主包（含 `dist/` 和 `types/`）。已发布的同版本包会跳过，
可以重跑失败的工作流。npm 侧需为六个包配置此工作流的 trusted publishing，
或提供具有发布权限的 `NODE_AUTH_TOKEN` 仓库 secret。

```sh
npm version patch         # 或 pnpm release:patch；同步版本、提交并创建 vX.Y.Z tag
npm version minor         # 同一 version hook 也支持 minor/major 等 npm version 参数
node scripts/sync-version.mjs  # 仅按当前主包版本同步，不递增版本、不暂存或提交
pnpm test:release         # 在临时仓库中验证同步、提交和 tag
```

`patch-version` 保留为 `npm version patch` 的兼容别名。版本 hook 同步：

- 主包 `optionalDependencies` 和 `platform/*/package.json`，清理废弃平台依赖。
- `pnpm-workspace.yaml` 中平台包的 `minimumReleaseAgeExclude`，保留其他配置。
- `Cargo.toml` 和 `Cargo.lock` 中 compiler 自身的版本，不修改依赖版本。

`npm version` 的提交会包含这些同步文件；没有运行 `git push` 或自动创建 GitHub Release。
下一版本的平台包发布前尚不存在，因此 hook 不刷新 `pnpm-lock.yaml`。
发布完成后运行 `pnpm install --lockfile-only` 并提交 lock 更新；在此之前，新增版本的
清单和旧 lock 不一致，`pnpm install --frozen-lockfile` 会失败。
主包发布任务在平台包发布完成后使用 `--no-frozen-lockfile` 安装构建依赖。
compiler 测试 CI 同样允许更新 lock，未发布的平台包作为 optional dependency 可跳过，
测试会从源码构建本地 binding；同级 jinge 的安装仍使用 frozen lock。

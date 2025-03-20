# jinge compiler

> 使用 rust 编写的 [jinge](https://github.com/jingeweb/jinge) 框架的模板编译器

## Develop

## Todo

- 支持比如 `<p>{a?.b?['slot:x']}</p>` 的 Optional-Chain 写法的插槽渲染，以及透传。
- 插槽也通过 props 参数传递，不用在 host 上通过 `SLOTS` 和 `DEFAULT_SLOTS` 这种 symbol 传递。
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

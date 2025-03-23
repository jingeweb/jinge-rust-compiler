# jinge compiler

> 使用 rust 编写的 [jinge](https://github.com/jingeweb/jinge) 框架的模板编译器

## Develop

## Todo

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

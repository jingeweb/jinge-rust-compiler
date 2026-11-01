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

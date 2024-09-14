# jinge compiler

> 使用 rust 编写的 [jinge](https://github.com/jingeweb/jinge) 框架的模板编译器

## Develop

## TODO

- Slot 渲染支持默认渲染内容，比如：
  ```tsx
  function A(props: PropsWithOptionalSlots<{}, JNode>) {
    return <div>{props.children ?? <div>Default slot</div>}</div>;
  }
  function B(props: PropsWithOptionalSlots<{}, (vm: any) => JNode>) {
    return <div>{props.children ? props.children() : <div>Default slot</div>}</div>;
  }
  function C(props: PropsWithSlots<{}, { a?: (vm: any) => JNode }>) {
    return <div>{props.children.a ? props.children.a() : <div>Default slot</div>}</div>;
  }
  ```

# jinge compiler

> 使用 rust 编写的 [jinge](https://github.com/jingeweb/jinge) 框架的模板编译器

## Develop

## TODO

- Slot 渲染支持布尔表达式判定，从而支持默认渲染内容或当传递了 slot 时才渲染局部，比如：

```tsx
import { Props } from 'jinge';
function A(props: Props<{
  children?: JNode
}>) {
  return <div>{props.children ?? <div>Default slot</div>}</div>;
}
function B(props: Props<{
  children?: () => JNode
}>) {
  return <div>{props.children ? props.children() : <div>Default slot</div>}</div>;
}
function C(props: Props<{
  slots: {
    a?: () => JNode
  }
}>) {
  return <div>{props['slot:a'] ? <p>Passed slot: {props['slot:a']}</p> : <div>Default slot</div>}</div>;
}
function D(props: Props<{
  slots: {
    a?: JNode
  }
}>) {
  return <div>{props['slot:a'] && <p>Some content with slot {props['slot:a']}</p>}</div>
}
```

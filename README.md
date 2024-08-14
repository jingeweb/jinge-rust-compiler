# jinge swc compiler

## Develop

### 环境准备

注意：当前暂时仅支持在 mac 或 windows 下构建（交叉编译构建 linux 和 windows 平台）。

TODO: 使用 Github Actions 跨平台构建并自动发布 npm 包。

准备环境：

1. 安装 `rust` 和 `node` 环境。
2. 安装 `rust` 交叉编译的环境。

   - `mac` 平台下：

   ```bash
   # windows 交叉编译
   rustup target add x86_64-pc-windows-gnu
   brew install mingw-w64
   # linux 交叉编译
   rustup target add x86_64-unknown-linux-musl
   brew install FiloSottile/musl-cross/musl-cross
   ```

## TODO

- Related Component 的完善和测试
- 二元条件表达式转成 If 组件，其中如果是 true/false 对应的都是不带需要 watch 的变量的表达式则转成内置代码（不使用 If 组件）
- For 组件的实现
- map 语句转成 For 组件。

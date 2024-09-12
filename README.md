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

- Optional Chain 表达式处理
- 组件的属性支持 `{...props}` 的书写。但仅支持属性只有这一个 spread 属性，本质上就是透传父组件的 props 或 state。例如：`<C {...state} />` 合法，`<C {...state} a={10} />` 不合法。后者用复杂的方案也能支持，比如再引入一个中间 ViewModel，但目前看没必要。

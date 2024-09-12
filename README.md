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

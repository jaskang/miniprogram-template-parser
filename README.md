# `@napi-rs/package-template`

![https://github.com/napi-rs/package-template/actions](https://github.com/napi-rs/package-template/workflows/CI/badge.svg)

> Template project for writing node packages with napi-rs.

# Usage

1. Click **Use this template**.
2. **Clone** your project.
3. Run `pnpm install` to install dependencies.
4. Run `npx napi rename -n [name]` command under the project folder to rename your package.

## Install this test package

```
pnpm add @napi-rs/package-template
```

## Usage

### Build

After `pnpm build` command, you can see `package-template.[darwin|win32|linux].node` file in project root. This is the native addon built from [lib.rs](./src/lib.rs).

### Test

With [ava](https://github.com/avajs/ava), run `pnpm test` to testing native addon. You can also switch to another testing framework if you want.

### CI

With GitHub Actions, each commit and pull request will be built and tested automatically in [`node@18`, `node@20`] x [`macOS`, `Linux`, `Windows`] matrix. You will never be afraid of the native addon broken in these platforms.

### Release

Release native package is very difficult in old days. Native packages may ask developers who use it to install `build toolchain` like `gcc/llvm`, `node-gyp` or something more.

With `GitHub actions`, we can easily prebuild a `binary` for major platforms. And with `N-API`, we should never be afraid of **ABI Compatible**.

The other problem is how to deliver prebuild `binary` to users. Downloading it in `postinstall` script is a common way that most packages do it right now. The problem with this solution is it introduced many other packages to download binary that has not been used by `runtime codes`. The other problem is some users may not easily download the binary from `GitHub/CDN` if they are behind a private network (But in most cases, they have a private NPM mirror).

In this package, we choose a better way to solve this problem. We release different `npm packages` for different platforms. And add it to `optionalDependencies` before releasing the `Major` package to npm.

`NPM` will choose which native package should download from `registry` automatically. You can see [npm](./npm) dir for details. And you can also run `pnpm add @napi-rs/package-template` to see how it works.

## Develop requirements

- Install the latest `Rust`
- Install `Node.js@16+` which fully supported `Node-API`
- Run `corepack enable`

## Test in local

- pnpm
- pnpm build
- pnpm test

And you will see:

```bash
$ ava --verbose

  ✔ sync function from native code
  ✔ sleep function from native code (201ms)
  ─

  2 tests passed
✨  Done in 1.12s.
```

## Release package

Ensure you have set your **NPM_TOKEN** in the `GitHub` project setting.

In `Settings -> Secrets`, add **NPM_TOKEN** into it.

When you want to release the package:

```
npm version [<newversion> | major | minor | patch | premajor | preminor | prepatch | prerelease [--preid=<prerelease-id>] | from-git]

git push
```

GitHub actions will do the rest job for you.

# 微信小程序模板解析器

这是一个用 Rust 实现的微信小程序 WXML 模板解析器，可以将 WXML 模板解析为抽象语法树 (AST)。

## 功能特点

- 支持解析标准 WXML 元素、属性和结构
- 支持微信小程序特有的 `{{ }}` 表达式语法
- 支持表达式在属性值中的混合使用
- 提供详细的位置信息，便于错误提示和代码高亮
- 灵活的错误处理机制
- 高效的字符流处理
- 全中文注释，便于理解和学习

## 用法示例

```rust
use miniprogram_template_parser::{parse, Node, AttributeValue};

fn main() {
    // WXML 模板代码
    let wxml_content = r#"
    <view class="container">
      <text>Hello, {{name}}!</text>
      <button bindtap="{{onClick}}">点击我</button>
    </view>
    "#;

    // 解析 WXML
    let ast = parse(wxml_content);

    // 现在你可以遍历 AST，进行进一步处理
    println!("AST 节点数量: {}", ast.children.len());
}
```

## 支持的节点类型

- 元素节点 (`Node::Element`)：表示 WXML 中的各种标签
- 文本节点 (`Node::Text`)：表示标签之间的纯文本内容
- 表达式节点 (`Node::Expression`)：表示 `{{ }}` 形式的表达式
- 注释节点 (`Node::Comment`)：表示 HTML 注释

## 主要类型说明

- `Root`: AST 的根节点，包含所有顶层节点
- `Node`: 表示 AST 中的节点，可以是元素、文本、表达式或注释
- `Attribute`: 表示元素的属性
- `AttributeValue`: 表示属性的值，可以是文本或表达式
- `Position`: 表示节点在源代码中的位置信息

## 项目结构

- `src/ast.rs`: 定义 AST 的数据结构
- `src/parser.rs`: 实现核心解析逻辑
- `src/state.rs`: 实现解析状态和字符流处理
- `src/error.rs`: 定义错误类型和处理机制
- `src/helpers.rs`: 提供辅助函数

## 限制说明

- 目前不支持解析微信小程序的特定指令（如 `wx:for`）的语义，仅作为普通属性处理
- 不带引号的属性值中如果包含表达式，可能无法正确解析

## 如何贡献

欢迎提交 Pull Request 或 Issue 来改进这个项目。

## 许可证

[MIT 许可证](LICENSE)

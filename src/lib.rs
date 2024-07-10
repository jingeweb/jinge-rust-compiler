mod ast;
mod common;
mod config;
mod visitor;
mod parser;


use config::Config;
use swc_core::ecma::{
  ast::Program,
  transforms::testing::test_inline,
  visit::{as_folder, FoldWith},
};
use swc_core::plugin::metadata::*;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_ecma_parser::TsSyntax;
use visitor::TransformVisitor;

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
  let filename = _metadata
    .get_context(&TransformPluginMetadataContextKind::Filename)
    .expect("failed to get filename for jinge-swc-plugin");

  let cwd = _metadata
    .get_context(&TransformPluginMetadataContextKind::Cwd)
    .expect("failed to get cwd");

  if !filename.starts_with(&cwd) || !filename.ends_with(".tsx") {
    return program;
  }

  // let config = _metadata
  //   .get_transform_plugin_config()
  //   .expect("failed to get plugin config for jinge-swc-plugin");

  // println!("{} {}", cwd, filename);

  // // 注意此处 filename 的获取方式需要和 `packages/tools/intl/extract.ts` 中的算法一致，如果修改两处都要变更。
  // let filename = filename[cwd.len()..].to_string();

  // // println!("START ... {}", filename);

  // // println!("CONFIG STR: {}", config);
  // let config =
  //   serde_json::from_str::<Config>(&config).expect("invalid config for binfoe-studio-swc-plugin");

  let t = TransformVisitor::new();

  program.fold_with(&mut as_folder(t))
}

// An example to test plugin transform.
// Recommended strategy to test plugin's transform is verify
// the Visitor's behavior, instead of trying to run `process_transform` with mocks
// unless explicitly required to do so.
test_inline!(
  swc_ecma_parser::Syntax::Typescript(swc_ecma_parser::TsSyntax {
    tsx: true,
    decorators: false,
    dts: true,
    no_early_errors: false,
    disallow_ambiguous_jsx_like: false
  }),
  |_| as_folder(TransformVisitor::new()),
  boo,
  // Input codes
  r#"
  import { Component } from 'jinge';
  export class App extends Component {
    render() {
      return <div><h1>hello</h1></div>;
    }
  }
  "#,
  // Output codes after transformed with plugin
  r#""#
);

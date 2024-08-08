mod ast;
mod common;
mod config;
mod parser;
mod visitor;

// use swc_core::ecma::{
//   ast::Program,
//   transforms::testing::test_inline,
//   visit::{as_folder, FoldWith},
// };
// use swc_core::plugin::metadata::*;
// use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

use neon::prelude::*;

// #[plugin_transform]
// pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
//   let filename = _metadata
//     .get_context(&TransformPluginMetadataContextKind::Filename)
//     .expect("failed to get filename for jinge-swc-plugin");

//   let cwd = _metadata
//     .get_context(&TransformPluginMetadataContextKind::Cwd)
//     .expect("failed to get cwd");

//   if !filename.starts_with(&cwd) || !filename.ends_with(".tsx") {
//     return program;
//   }

//   // let config = _metadata
//   //   .get_transform_plugin_config()
//   //   .expect("failed to get plugin config for jinge-swc-plugin");

//   // println!("{} {}", cwd, filename);

//   // // 注意此处 filename 的获取方式需要和 `packages/tools/intl/extract.ts` 中的算法一致，如果修改两处都要变更。
//   // let filename = filename[cwd.len()..].to_string();

//   // // println!("START ... {}", filename);

//   // // println!("CONFIG STR: {}", config);
//   // let config =
//   //   serde_json::from_str::<Config>(&config).expect("invalid config for binfoe-studio-swc-plugin");

//   let t = TransformVisitor::new();

//   program.fold_with(&mut as_folder(t))
// }
use swc_common::{
  comments::SingleThreadedComments,
  errors::{ColorConfig, Handler},
  sync::Lrc,
  BytePos, Globals, Mark, SourceFile, SourceMap, GLOBALS,
};
use swc_ecma_codegen::to_code_default;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::{as_folder, FoldWith};
use visitor::TransformVisitor;

fn inner_transform(code: &str) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
    Default::default(),
    StringInput::new(code, BytePos(0), BytePos(code.len() as u32)),
    None,
  );

  let mut parser = Parser::new_from(lexer);

  for e in parser.take_errors() {
    e.into_diagnostic(&handler).emit();
  }

  let module = parser
    .parse_program()
    .map_err(|e| e.into_diagnostic(&handler).emit())
    .expect("failed to parse module.");

  let globals = Globals::default();
  let output = GLOBALS.set(&globals, || {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    let t = TransformVisitor::new();

    // Optionally transforms decorators here before the resolver pass
    // as it might produce runtime declarations.

    // Conduct identifier scope analysis
    // let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

    // Remove typescript types
    let module = module.fold_with(&mut strip(unresolved_mark, top_level_mark));
    let module = module.fold_with(&mut as_folder(t));

    // Fix up any identifiers with the same name, but different contexts
    // let module = module.fold_with(&mut hygiene());

    // Ensure that we have enough parenthesis.
    let module = module.fold_with(&mut fixer(None));

    to_code_default(cm, None, &module)
  });
  output
}

fn transform(mut cx: FunctionContext) -> JsResult<JsObject> {
  let origin_code = cx.argument::<JsString>(0)?;
  let origin_code = origin_code.value(&mut cx);
  let output_code = inner_transform(&origin_code);
  let obj = cx.empty_object();
  let obj_code = cx.string(output_code);
  obj.set(&mut cx, "code", obj_code)?;
  Ok(obj)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
  cx.export_function("transform", transform)?;
  Ok(())
}

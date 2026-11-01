#![recursion_limit = "256"]

mod ast;
mod common;
mod helper;
mod parser;
mod scope;
mod visitor;

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use common::IntlType;
use neon::prelude::*;

use swc_common::input::SourceFileInput;
use swc_common::source_map::DefaultSourceMapGenConfig;
use swc_common::{
  FileName, GLOBALS, Globals, Mark, SourceMap,
  errors::{HANDLER, Handler},
  sync::Lrc,
};
use swc_core::ecma::ast::EsVersion;
use swc_ecma_codegen::{Emitter, Node, text_writer::JsWriter};
use swc_ecma_parser::{Parser, Syntax, TsSyntax, lexer::Lexer};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::visit_mut_pass;
use visitor::TemplateTransformVisitor;

fn print(
  cm: Lrc<SourceMap>,
  node: &impl Node,
  sourcemap_enabled: bool,
  // names: &AHashMap<BytePos, swc_core::atoms::JsWord>,
) -> (String, Option<String>) {
  let mut src_map_buf = Vec::new();
  let src = {
    let mut buf = Vec::new();
    {
      let mut emitter = Emitter {
        cfg: Default::default(),
        cm: cm.clone(),
        comments: None,
        wr: JsWriter::new(
          cm.clone(),
          "\n",
          &mut buf,
          if sourcemap_enabled {
            Some(&mut src_map_buf)
          } else {
            None
          },
        ),
      };
      node.emit_with(&mut emitter).unwrap();
    }

    String::from_utf8(buf).expect("codegen generated non-utf8 output")
  };
  let map = if sourcemap_enabled {
    let map = cm.build_source_map(&src_map_buf, None, DefaultSourceMapGenConfig);
    let mut buf = Vec::new();

    map
      .to_writer(&mut buf)
      .expect("source map to writer failed");
    Some(String::from_utf8(buf).expect("source map is not utf-8"))
  } else {
    None
  };
  // println!("{:?}", map);
  (src, map)
}

///
/// intl_type 国际化类型： 0： 不启用国际化，1： 启用国际化，保留原始文本，2：启用国际化，去除原始文本。
#[derive(Clone, Default)]
struct DiagnosticWriter(Arc<Mutex<Vec<u8>>>);
impl Write for DiagnosticWriter {
  fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
    self.0.lock().unwrap().extend_from_slice(bytes);
    Ok(bytes.len())
  }
  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

fn inner_transform(
  filename: String,
  code_type: usize,
  code: String,
  sourcemap_enabled: bool,
  intl_type: IntlType,
) -> Result<(String, String, Option<String>), String> {
  // let code = Lrc::new(code);
  let cm: Arc<SourceMap> = Arc::<SourceMap>::default();
  let fm = cm.new_source_file(Arc::new(FileName::from(PathBuf::from(&filename))), code);
  let diagnostics = DiagnosticWriter::default();
  let handler = Handler::with_emitter_writer(Box::new(diagnostics.clone()), Some(cm.clone()));

  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: code_type == 2,
      ..Default::default()
    }),
    EsVersion::latest(),
    SourceFileInput::from(&*fm),
    None,
  );

  let mut parser = Parser::new_from(lexer);

  let module = parser.parse_program();
  for e in parser.take_errors() {
    e.into_diagnostic(&handler).emit();
  }
  let module = match module {
    Ok(module) if !handler.has_errors() => module,
    result => {
      if let Err(e) = result {
        e.into_diagnostic(&handler).emit();
      }
      return Err(String::from_utf8_lossy(&diagnostics.0.lock().unwrap()).into_owned());
    }
  };

  let result = GLOBALS.set(&Globals::default(), || {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    // https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples/ts_to_js.rs
    // Conduct identifier scope analysis
    let module = module.apply(resolver(unresolved_mark, top_level_mark, true));
    let _scope = scope::ScopeGuard::enter(&module, code_type == 2);
    // Remove typescript types
    let module = module.apply(strip(unresolved_mark, top_level_mark));

    HANDLER.set(&handler, move || {
      let mut parsed_components: Vec<String> = vec![];

      let module = if code_type == 2 {
        // 只有 tsx 类型才需要转换
        let t = TemplateTransformVisitor::new(&mut parsed_components, intl_type);
        // module.fold_with(&mut as_folder(t))
        module.apply(visit_mut_pass(t))
      } else {
        module.apply(visit_mut_pass(visitor::IntlTransformVisitor { intl_type }))
      };

      // let module = if let IntlType::Enabled(drop_default_text) = intl_type {
      //   let t = IntlTransformVisitor::new(drop_default_text);
      //   module.apply(visit_mut_pass(t))
      // } else {
      //   module
      // };

      use swc_ecma_visit::VisitWith;
      module.visit_with(&mut visitor::RemainingJsx);
      // Fix up any identifiers with the same name, but different contexts
      // let module = module.apply(hygiene());
      // Ensure that we have enough parenthesis.
      let module = module.apply(fixer(None));

      // https://github.com/swc-project/swc/blob/main/crates/swc_ecma_codegen/examples/sourcemap.rs
      let (code, map) = print(cm, &module, sourcemap_enabled);

      (code, parsed_components.join(","), map)
    })
  });
  let messages = String::from_utf8_lossy(&diagnostics.0.lock().unwrap()).into_owned();
  if handler.has_errors() {
    Err(messages)
  } else {
    if !messages.is_empty() {
      eprint!("{messages}");
    }
    Ok(result)
  }
}

fn transform(mut cx: FunctionContext) -> JsResult<JsObject> {
  let file_name = cx.argument::<JsString>(0)?.value(&mut cx);
  let code_type = cx.argument::<JsNumber>(1)?.value(&mut cx) as usize;
  let origin_code = cx.argument::<JsString>(2)?.value(&mut cx);
  let sourcemap_enabled = cx.argument::<JsBoolean>(3)?.value(&mut cx);
  let intl_type = cx.argument::<JsNumber>(4)?.value(&mut cx) as u8;
  // let hmr_enabled = cx.argument::<JsBoolean>(3)?.value(&mut cx);
  let result = inner_transform(
    file_name,
    code_type,
    origin_code,
    sourcemap_enabled,
    if intl_type == 0 {
      IntlType::Disabled
    } else {
      IntlType::Enabled(intl_type > 1)
    },
  );
  let (code, parsed_components, map) = match result {
    Ok(result) => result,
    Err(message) => return cx.throw_error(message),
  };
  let obj = cx.empty_object();
  let obj_code = cx.string(code);
  let obj_map = cx.string(map.unwrap_or("".into()));
  let parsed_components = cx.string(parsed_components);
  obj.set(&mut cx, "code", obj_code)?;
  obj.set(&mut cx, "map", obj_map)?;
  obj.set(&mut cx, "parsedComponents", parsed_components)?;
  Ok(obj)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
  cx.export_function("transform", transform)?;
  Ok(())
}

#[cfg(test)]
#[path = "../test/rust/mod.rs"]
mod tests;

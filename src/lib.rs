mod ast;
mod common;
mod parser;
mod visitor;

// use swc_core::ecma::{
//   ast::Program,
//   transforms::testing::test_inline,
//   visit::{as_folder, FoldWith},
// };
// use swc_core::plugin::metadata::*;
// use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

use std::path::PathBuf;

use neon::prelude::*;

use swc_common::{
  collections::AHashMap,
  errors::{ColorConfig, Handler, HANDLER},
  source_map::SourceMapGenConfig,
  sync::Lrc,
  BytePos, FileName, Globals, Mark, SourceMap, GLOBALS,
};
use swc_core::ecma::ast::{Ident, IdentName};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter, Node};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::{as_folder, noop_visit_type, FoldWith, Visit, VisitWith};
use visitor::TransformVisitor;

struct SourceMapConfig<'a> {
  filename: &'a str,
  names: &'a AHashMap<BytePos, swc_core::atoms::JsWord>,
}
impl SourceMapGenConfig for SourceMapConfig<'_> {
  fn file_name_to_source(&self, _: &FileName) -> String {
    self.filename.to_string()
  }
  fn inline_sources_content(&self, _: &FileName) -> bool {
    true
  }
  fn name_for_bytepos(&self, pos: BytePos) -> Option<&str> {
    self.names.get(&pos).map(|v| &**v)
  }
}

pub struct IdentCollector {
  pub names: AHashMap<BytePos, swc_core::atoms::JsWord>,
}

impl Visit for IdentCollector {
  noop_visit_type!();

  fn visit_ident(&mut self, ident: &Ident) {
    self.names.insert(ident.span.lo, ident.sym.clone());
  }

  fn visit_ident_name(&mut self, ident: &IdentName) {
    self.names.insert(ident.span.lo, ident.sym.clone());
  }
}

fn print(
  filename: &str,
  cm: Lrc<SourceMap>,
  node: &impl Node,
  enable_source_map: bool,
  names: &AHashMap<BytePos, swc_core::atoms::JsWord>,
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
          if enable_source_map {
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
  let map = if enable_source_map {
    let map = cm.build_source_map_with_config(
      &src_map_buf,
      None,
      SourceMapConfig {
        filename: filename,
        names,
      },
    );
    let mut buf = Vec::new();

    map
      .to_writer(&mut buf)
      .expect("source map to writer failed");
    Some(String::from_utf8(buf).expect("source map is not utf-8"))
  } else {
    None
  };
  (src, map)
}

fn inner_transform(filename: String, code: String) -> (String, Option<String>) {
  // let code = Lrc::new(code);
  let cm: Lrc<SourceMap> = Lrc::<SourceMap>::default();
  let fm = cm.new_source_file(Lrc::new(FileName::from(PathBuf::from(&filename))), code);
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
  // HANDLER.set(t, f)
  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
    Default::default(),
    StringInput::new(&fm.src, BytePos(0), BytePos(fm.src.len() as u32)),
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

  let output = GLOBALS.set(&Globals::default(), || {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    // Optionally transforms decorators here before the resolver pass
    // as it might produce runtime declarations.

    // Conduct identifier scope analysis
    // let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

    // Remove typescript types
    let module = module.fold_with(&mut strip(unresolved_mark, top_level_mark));

    HANDLER.set(&handler, move || {
      let t = TransformVisitor::new();
      let module = module.fold_with(&mut as_folder(t));
      // Fix up any identifiers with the same name, but different contexts
      // let module = module.fold_with(&mut hygiene());

      // Ensure that we have enough parenthesis.
      let module = module.fold_with(&mut fixer(None));

      let source_map_names = if true {
        let mut v = IdentCollector {
          names: Default::default(),
        };

        module.visit_with(&mut v);

        v.names
      } else {
        Default::default()
      };
      print(&filename, cm, &module, true, &source_map_names)
    })
  });
  output
}

fn transform(mut cx: FunctionContext) -> JsResult<JsObject> {
  let file_name = cx.argument::<JsString>(0)?.value(&mut cx);
  let origin_code = cx.argument::<JsString>(1)?;
  let origin_code = origin_code.value(&mut cx);
  let (code, map) = inner_transform(file_name, origin_code);
  let obj = cx.empty_object();
  let obj_code = cx.string(code);
  let obj_map = cx.string(map.unwrap_or("".into()));
  obj.set(&mut cx, "code", obj_code)?;
  obj.set(&mut cx, "map", obj_map)?;
  Ok(obj)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
  // println!("rust core loaded");
  cx.export_function("transform", transform)?;
  Ok(())
}

#[test]
fn test_transform() {
  let (code, _) = inner_transform(
    "test.tsx".into(),
    "export class C extends Component {
  render() {
    return this[SLOTS][DEFAULT_SLOT]?.(this, this); 
  }
}"
    .into(),
  );
  println!("{}", code);
  assert_eq!(code, "x");
}

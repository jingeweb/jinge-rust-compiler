mod ast;
mod common;
mod parser;
mod visitor;

use std::path::PathBuf;
use std::sync::Arc;

use common::IntlType;
use neon::prelude::*;

use swc_common::input::SourceFileInput;
use swc_common::{
  collections::AHashMap,
  errors::{ColorConfig, Handler, HANDLER},
  source_map::SourceMapGenConfig,
  sync::Lrc,
  BytePos, FileName, Globals, Mark, SourceMap, GLOBALS,
};
use swc_core::ecma::ast::{EsVersion, Ident, IdentName};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter, Node};
use swc_ecma_parser::{lexer::Lexer, Parser, Syntax, TsSyntax};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::{as_folder, noop_visit_type, FoldWith, Visit, VisitWith};
use visitor::{IntlTransformVisitor, TemplateTransformVisitor};

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
  sourcemap_enabled: bool,
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
  // println!("{}", src);
  (src, map)
}

///
/// intl_type 国际化类型： 0： 不启用国际化，1： 启用国际化，保留原始文本，2：启用国际化，去除原始文本。
fn inner_transform(
  filename: String,
  code_type: usize,
  code: String,
  sourcemap_enabled: bool,
  intl_type: IntlType,
) -> (String, String, Option<String>) {
  // let code = Lrc::new(code);
  let cm: Arc<SourceMap> = Arc::<SourceMap>::default();
  let fm = cm.new_source_file(Arc::new(FileName::from(PathBuf::from(&filename))), code);
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
  // HANDLER.set(t, f)
  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
    EsVersion::latest(),
    SourceFileInput::from(&*fm),
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

  GLOBALS.set(&Globals::default(), || {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    // Remove typescript types
    let module = module.fold_with(&mut strip(unresolved_mark, top_level_mark));

    HANDLER.set(&handler, move || {
      let mut parsed_components: Vec<String> = vec![];

      let module = if code_type == 2 {
        // 只有 tsx 类型才需要转换
        let t = TemplateTransformVisitor::new(&mut parsed_components, intl_type);
        module.fold_with(&mut as_folder(t))
      } else {
        // Ensure that we have enough parenthesis.
        module
      };

      let module = if let IntlType::Enabled(drop_default_text) = intl_type {
        let t = IntlTransformVisitor::new(drop_default_text);
        module.fold_with(&mut as_folder(t))
      } else {
        module
      };

      let module = module.fold_with(&mut fixer(None));

      let source_map_names = if sourcemap_enabled {
        let mut v = IdentCollector {
          names: Default::default(),
        };

        module.visit_with(&mut v);

        v.names
      } else {
        Default::default()
      };
      let (code, map) = print(&filename, cm, &module, sourcemap_enabled, &source_map_names);

      (code, parsed_components.join(","), map)
    })
  })
}

fn transform(mut cx: FunctionContext) -> JsResult<JsObject> {
  let file_name = cx.argument::<JsString>(0)?.value(&mut cx);
  let code_type = cx.argument::<JsNumber>(1)?.value(&mut cx) as usize;
  let origin_code = cx.argument::<JsString>(2)?.value(&mut cx);
  let sourcemap_enabled = cx.argument::<JsBoolean>(3)?.value(&mut cx);
  let intl_type = cx.argument::<JsNumber>(4)?.value(&mut cx) as u8;
  // let hmr_enabled = cx.argument::<JsBoolean>(3)?.value(&mut cx);
  let (code, parsed_components, map) = inner_transform(
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
  // println!("rust core loaded");
  cx.export_function("transform", transform)?;
  Ok(())
}

#[test]
fn test_transform() {
  let (code, parsed_components, _) = inner_transform(
    "test.tsx".into(),
    2,
    "const $jg$ = (src: string, content: string) => src.replace('{:?}', content);
export default {
  XKVhbP: ({ name }: Record<string, string>) => `你好，${name}`,
  m3HSJL: () => '你好',
  '7fxvwR': ({ name, red, b }: Record<string, string>) =>
    `你好，${$jg$(red, `${$jg$(b, `哦哦`)}：${name}`)}`,
};"
      .into(),
    true,
    IntlType::Disabled,
  );
  println!("PARSED COMPONENTS: {}", parsed_components);
  std::fs::write("target/out.ts", &code).unwrap();
  // println!("{:#?}", code);
  // assert_eq!(code, "x");
  // assert!(false)
}

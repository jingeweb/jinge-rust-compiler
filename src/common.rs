use swc_core::common::errors::HANDLER;
use swc_core::common::{Span, DUMMY_SP};
use swc_core::ecma::ast::{
  Ident, ImportDecl, ImportNamedSpecifier, ImportPhase, ImportSpecifier, ModuleDecl,
  ModuleExportName, ModuleItem, Str,
};

macro_rules! x {
  ($name:literal) => {
    ($name, concat!($name, "$jg$"))
  };
}

// TODO: should use macro to generate

pub const JINGE_IMPORT_TEXT_RENDER_FN: (&str, &str) = x!("textRenderFn");
pub const JINGE_IMPORT_CREATE_ELE: (&str, &str) = x!("createEle");
pub const JINGE_IMPORT_CREATE_ELE_A: (&str, &str) = x!("createEleA");
pub const JINGE_IMPORT_ADD_EVENT: (&str, &str) = x!("addEvent");
pub const JINGE_IMPORT_SET_ATTRIBUTE: (&str, &str) = x!("setAttribute");
pub const JINGE_IMPORT_PATH_WATCHER: (&str, &str) = x!("PathWatcher");
pub const JINGE_IMPORT_DYM_PATH_WATCHER: (&str, &str) = x!("DymPathWatcher");
pub const JINGE_IMPORT_EXPR_WATCHER: (&str, &str) = x!("ExprWatcher");
pub const JINGE_IMPORT_WATCH_FOR_COMPONENT: (&str, &str) = x!("watchForComponent");

pub const JINGE_IMPORT_SET_REF: (&str, &str) = x!("setRefForComponent");
pub const JINGE_IMPORT_ROOT_NODES: (&str, &str) = x!("ROOT_NODES");
pub const JINGE_IDENT: &str = "$jg$";

const JINGE_IMPORTS: [(&str, &str); 11] = [
  JINGE_IMPORT_TEXT_RENDER_FN,
  JINGE_IMPORT_CREATE_ELE,
  JINGE_IMPORT_CREATE_ELE_A,
  JINGE_IMPORT_ADD_EVENT,
  JINGE_IMPORT_SET_ATTRIBUTE,
  JINGE_IMPORT_SET_REF,
  JINGE_IMPORT_ROOT_NODES,
  JINGE_IMPORT_PATH_WATCHER,
  JINGE_IMPORT_DYM_PATH_WATCHER,
  JINGE_IMPORT_EXPR_WATCHER,
  JINGE_IMPORT_WATCH_FOR_COMPONENT,
];

pub fn gen_import_jinge() -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: JINGE_IMPORTS
      .iter()
      .map(|imp| {
        ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: Ident::from(imp.1),
          imported: Some(ModuleExportName::Ident(Ident::from(imp.0))),
          is_type_only: false,
        })
      })
      .collect(),
    src: Box::new(Str::from("jinge")),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }))
}

pub fn emit_error(sp: Span, msg: &str) {
  HANDLER.with(|h| {
    h.struct_span_err(sp, msg).emit();
  });
}

lazy_static::lazy_static! {
  pub static ref IDL_ATTRIBUTE_SET: Vec<&'static str> = {
    let mut attrs = vec!["disabled", "readOnly", "autoFocus", "autoComplete", "autoPlay", "controls", "required", "checked", "selected", "multiple", "muted", "draggable"];
    attrs.sort_unstable();
    attrs
  };
}

use swc_core::atoms::Atom;
use swc_core::common::errors::HANDLER;
use swc_core::common::{Span, DUMMY_SP};
use swc_core::ecma::ast::*;

pub struct JingeImport {
  local: Ident,
  imported: Ident,
}
impl JingeImport {
  #[inline]
  fn new(imported: &'static str, local: &'static str) -> Self {
    Self {
      local: local.into(),
      imported: imported.into(),
    }
  }
  #[inline]
  pub fn local(&self) -> Ident {
    self.local.clone()
  }
  #[inline]
  pub fn imported(&self) -> Ident {
    self.imported.clone()
  }
}

// A macro which uses repetitions
macro_rules! x {
  // match rule which matches multiple expressions in an argument
  ( $x:literal) => {
    JingeImport::new($x, concat!($x, "$jg$"))
  };
}

// TODO: should use macro to generate
lazy_static::lazy_static! {
  pub static ref JINGE_IMPORT_TEXT_RENDER_FN: JingeImport = x!("textRenderFn");
  pub static ref JINGE_IMPORT_CREATE_ELE: JingeImport = x!("createEle");
  pub static ref JINGE_IMPORT_CREATE_ELE_A: JingeImport = x!("createEleA");
  pub static ref JINGE_IMPORT_ADD_EVENT: JingeImport = x!("addEvent");
  pub static ref JINGE_IMPORT_SET_ATTRIBUTE: JingeImport = x!("setAttribute");
  pub static ref JINGE_IMPORT_IF: JingeImport = x!("If");
  pub static ref JINGE_IMPORT_PATH_WATCHER: JingeImport = x!("PathWatcher");
  pub static ref JINGE_IMPORT_DYM_PATH_WATCHER: JingeImport = x!("DymPathWatcher");
  pub static ref JINGE_IMPORT_EXPR_WATCHER: JingeImport = x!("ExprWatcher");
  pub static ref JINGE_IMPORT_WATCH_FOR_COMPONENT: JingeImport = x!("watchForComponent");
  pub static ref JINGE_IMPORT_VM: JingeImport = x!("vm");
  pub static ref JINGE_IMPORT_SET_REF: JingeImport = x!("setRefForComponent");
  pub static ref JINGE_IMPORT_ROOT_NODES: JingeImport = x!("ROOT_NODES");
  pub static ref JINGE_IMPORT_NEW_COM_SLOTS: JingeImport = x!("newComponentWithSlots");
  pub static ref JINGE_IMPORT_NEW_COM_DEFAULT_SLOT: JingeImport = x!("newComponentWithDefaultSlot");

  pub static ref JINGE_IMPORT_NON_ROOT_COMPONENT_NODES: JingeImport = x!("NON_ROOT_COMPONENT_NODES");
  pub static ref JINGE_EL_IDENT: Ident = "$jg$".into();
  pub static ref JINGE_ATTR_IDENT: Ident = "attrs$jg$".into();
  pub static ref JINGE_V_IDENT: Ident = "v".into();
  pub static ref JINGE_HOST_IDENT: Ident = "host$jg$".into();
  pub static ref IDL_ATTRIBUTE_SET: Vec<Atom> = {
    let mut attrs = vec!["disabled", "readOnly", "autoFocus", "autoComplete", "autoPlay", "controls", "required", "checked", "selected", "multiple", "muted", "draggable"];
    attrs.sort_unstable();
    attrs.into_iter().map(|s| Atom::from(s)).collect()
  };
}

pub fn gen_import_jinge() -> ModuleItem {
  let imports: [&'static JingeImport; 16] = [
    &JINGE_IMPORT_TEXT_RENDER_FN,
    &JINGE_IMPORT_CREATE_ELE,
    &JINGE_IMPORT_CREATE_ELE_A,
    &JINGE_IMPORT_VM,
    &JINGE_IMPORT_ADD_EVENT,
    &JINGE_IMPORT_SET_ATTRIBUTE,
    &JINGE_IMPORT_SET_REF,
    &JINGE_IMPORT_ROOT_NODES,
    &JINGE_IMPORT_NON_ROOT_COMPONENT_NODES,
    &JINGE_IMPORT_NEW_COM_SLOTS,
    &JINGE_IMPORT_NEW_COM_DEFAULT_SLOT,
    &JINGE_IMPORT_PATH_WATCHER,
    &JINGE_IMPORT_DYM_PATH_WATCHER,
    &JINGE_IMPORT_EXPR_WATCHER,
    &JINGE_IMPORT_WATCH_FOR_COMPONENT,
    &JINGE_IMPORT_IF,
  ];
  let specs: Vec<_> = imports
    .map(|e| {
      ImportSpecifier::Named(ImportNamedSpecifier {
        span: DUMMY_SP,
        local: e.local(),
        imported: Some(ModuleExportName::Ident(e.imported())),
        is_type_only: false,
      })
    })
    .into_iter()
    .collect();
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: specs,
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

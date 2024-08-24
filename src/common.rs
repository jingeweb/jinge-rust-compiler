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
  pub static ref JINGE_IMPORT_CREATE_TEXT_NODE: JingeImport = x!("createTextNode");

  pub static ref JINGE_IMPORT_CREATE_ELE_A: JingeImport = x!("createEleA");
  pub static ref JINGE_IMPORT_ADD_EVENT: JingeImport = x!("addEvent");
  pub static ref JINGE_IMPORT_SET_ATTRIBUTE: JingeImport = x!("setAttribute");
  pub static ref JINGE_IMPORT_SET_TEXT_CONTENT: JingeImport = x!("setTextContent");
  pub static ref JINGE_IMPORT_IF: JingeImport = x!("If");
  pub static ref JINGE_IMPORT_PATH_WATCHER: JingeImport = x!("PathWatcher");
  pub static ref JINGE_IMPORT_DYM_PATH_WATCHER: JingeImport = x!("DymPathWatcher");
  pub static ref JINGE_IMPORT_EXPR_WATCHER: JingeImport = x!("ExprWatcher");
  pub static ref JINGE_IMPORT_WATCH_FOR_RENDER: JingeImport = x!("watchForRender");
  pub static ref JINGE_IMPORT_WATCH_PATH_FOR_RENDER_2: JingeImport = x!("watchPathForRender2");
  pub static ref JINGE_IMPORT_WATCH_PATH_FOR_RENDER: JingeImport = x!("watchPathForRender");
  pub static ref JINGE_IMPORT_CONTEXT: JingeImport = x!("CONTEXT");

  pub static ref JINGE_IMPORT_VM: JingeImport = x!("vm");
  pub static ref JINGE_IMPORT_SET_REF: JingeImport = x!("setRefForComponent");
  pub static ref JINGE_IMPORT_ROOT_NODES: JingeImport = x!("ROOT_NODES");
  pub static ref JINGE_IMPORT_SLOTS: JingeImport = x!("SLOTS");
  pub static ref JINGE_IMPORT_DEFAULT_SLOT: JingeImport = x!("DEFAULT_SLOT");
  pub static ref JINGE_IMPORT_NEW_COM_SLOTS: JingeImport = x!("newComponentWithSlots");
  pub static ref JINGE_IMPORT_NEW_COM_DEFAULT_SLOT: JingeImport = x!("newComponentWithDefaultSlot");
  pub static ref JINGE_IMPORT_NEW_SLOT_RENDER_COM: JingeImport = x!("newSlotRenderComponent");

  pub static ref JINGE_IMPORT_NON_ROOT_COMPONENT_NODES: JingeImport = x!("NON_ROOT_COMPONENT_NODES");
  pub static ref JINGE_EL_IDENT: Ident = "$jg$".into();
  pub static ref JINGE_ATTR_IDENT: Ident = "attrs$jg$".into();
  pub static ref JINGE_V_IDENT: Ident = "v".into();
  pub static ref JINGE_HOST_IDENT: Ident = "host$jg$".into();
  pub static ref TEXT_CONTENT: Ident = "textContent".into();
  pub static ref JINGE: Atom = "jinge".into();
  pub static ref JINGE_RENDER: Atom = "render".into();
  pub static ref JINGE_SVG: Atom = "svg".into();
  pub static ref JINGE_SLOTS: Atom = "slots".into();

  pub static ref IDL_ATTRIBUTE_SET: Vec<Atom> = {
    let mut attrs = vec!["disabled", "readOnly", "autoFocus", "autoComplete", "autoPlay", "controls", "required", "checked", "selected", "multiple", "muted", "draggable"];
    attrs.sort_unstable();
    attrs.into_iter().map(|s| Atom::from(s)).collect()
  };

  pub static ref JINGE_IMPORT_MODULE_ITEM: ModuleItem = gen_import_jinge();
}

fn gen_import_jinge() -> ModuleItem {
  let imports: [&'static JingeImport; 24] = [
    &JINGE_IMPORT_TEXT_RENDER_FN,
    &JINGE_IMPORT_CREATE_ELE,
    &JINGE_IMPORT_CREATE_ELE_A,
    &JINGE_IMPORT_CONTEXT,
    &JINGE_IMPORT_CREATE_TEXT_NODE,
    &JINGE_IMPORT_SET_TEXT_CONTENT,
    &JINGE_IMPORT_VM,
    &JINGE_IMPORT_ADD_EVENT,
    &JINGE_IMPORT_SET_ATTRIBUTE,
    &JINGE_IMPORT_SET_REF,
    &JINGE_IMPORT_ROOT_NODES,
    &JINGE_IMPORT_SLOTS,
    &JINGE_IMPORT_DEFAULT_SLOT,
    &JINGE_IMPORT_NON_ROOT_COMPONENT_NODES,
    &JINGE_IMPORT_NEW_COM_SLOTS,
    &JINGE_IMPORT_NEW_COM_DEFAULT_SLOT,
    &JINGE_IMPORT_NEW_SLOT_RENDER_COM,
    &JINGE_IMPORT_PATH_WATCHER,
    &JINGE_IMPORT_DYM_PATH_WATCHER,
    &JINGE_IMPORT_EXPR_WATCHER,
    &JINGE_IMPORT_WATCH_FOR_RENDER,
    &JINGE_IMPORT_WATCH_PATH_FOR_RENDER,
    &JINGE_IMPORT_WATCH_PATH_FOR_RENDER_2,
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
    src: Box::new(Str::from(JINGE.clone())),
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

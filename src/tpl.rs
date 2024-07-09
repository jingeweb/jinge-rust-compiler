use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::{
  ast::{ast_create_expr_call, ast_create_expr_ident, ast_create_expr_this},
  common::{JINGE_IMPORTS, JINGE_IMPORT_TEXT_RENDER_FN},
};

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

pub fn gen_text_render_func(v: Box<Expr>) -> Box<Expr> {
  ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.1),
    vec![ExprOrSpread {
      spread: None,
      expr: v,
    }],
  )
}

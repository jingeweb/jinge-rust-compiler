use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::{
  ast::{ast_create_expr_call, ast_create_expr_ident, ast_create_expr_this},
  common::{JINGE_IMPORT_TEXT_RENDER_FN},
};



pub fn gen_text_render_func(v: Box<Expr>) -> Box<Expr> {
  ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.1),
    vec![ExprOrSpread {
      spread: None,
      expr: v,
    }],
  )
}

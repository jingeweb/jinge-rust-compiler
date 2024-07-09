use swc_core::{atoms::Atom, common::DUMMY_SP, ecma::ast::*};

use crate::ast::{ast_create_expr_ident, ast_create_expr_this};

pub fn text_render_func(v: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Call(CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(ast_create_expr_ident("jinge$textRenderFn$")),
    args: vec![
      ExprOrSpread {
        spread: None,
        expr: ast_create_expr_this(),
      },
      ExprOrSpread {
        spread: None,
        expr: v,
      },
    ],
    type_args: None,
  }))
}

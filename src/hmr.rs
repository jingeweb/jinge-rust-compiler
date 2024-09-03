use swc_common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;

use crate::ast::{ast_create_expr_ident, ast_create_expr_member};

lazy_static::lazy_static! {
  pub static ref JINGE_HMR_INJECT_CODE: Stmt = gen_hmr_inject_code();
}

fn gen_hmr_inject_code() -> Stmt {
  let stmts: Vec<Stmt> = vec![];

  Stmt::If(IfStmt {
    span: DUMMY_SP,
    test: ast_create_expr_member(
      Box::new(Expr::MetaProp(MetaPropExpr {
        span: DUMMY_SP,
        kind: MetaPropKind::ImportMeta,
      })),
      MemberProp::Ident(IdentName::from("hot")),
    ),
    cons: Box::new(Stmt::Block(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts,
    })),
    alt: None,
  })
}

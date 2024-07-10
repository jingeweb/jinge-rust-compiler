use swc_core::{common::*, ecma::ast::*, ecma::atoms::*};

#[inline]
pub fn ast_create_ident(id: &str) -> Ident {
  Ident {
    span: DUMMY_SP,
    sym: Atom::from(id),
    optional: false,
  }
}
#[inline]
pub fn ast_create_expr_ident(id: &str) -> Box<Expr> {
  Box::new(Expr::Ident(ast_create_ident(id)))
}
#[inline]
pub fn ast_create_expr_this() -> Box<Expr> {
  Box::new(Expr::This(ThisExpr { span: DUMMY_SP }))
}
#[inline]
pub fn ast_create_expr_lit_str(v: &str, sp: Option<Span>) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    span: sp.unwrap_or(DUMMY_SP),
    value: Atom::from(v),
    raw: None,
  })))
}
#[inline]
pub fn ast_create_expr_call(callee: Box<Expr>, args: Vec<ExprOrSpread>) -> Box<Expr> {
  Box::new(Expr::Call(CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(callee),
    args,
    type_args: None,
  }))
}
pub fn ast_create_console_log() -> ModuleItem {
  ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    span: DUMMY_SP,
    expr: Box::new(Expr::Call(CallExpr {
      span: DUMMY_SP,
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(Ident {
          span: DUMMY_SP,
          sym: Atom::from("console"),
          optional: false,
        })),
        prop: MemberProp::Ident(Ident {
          span: DUMMY_SP,
          sym: Atom::from("log"),
          optional: false,
        }),
      }))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: Atom::from("hello"),
          raw: None,
        }))),
      }],
      type_args: None,
    })),
  }))
}

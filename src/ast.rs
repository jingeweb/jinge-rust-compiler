use swc_core::{common::*, ecma::ast::*, ecma::atoms::*};

#[inline]
pub fn ast_create_ident(id: &str) -> Ident {
  Ident {
    ctxt: SyntaxContext::empty(),
    span: DUMMY_SP,
    sym: Atom::from(id),
    optional: false,
  }
}
#[inline]
pub fn ast_create_arg_expr(arg: Box<Expr>) -> ExprOrSpread {
  ExprOrSpread {
    spread: None,
    expr: arg,
  }
}
#[inline]
pub fn ast_create_expr_this() -> Box<Expr> {
  Box::new(Expr::This(ThisExpr { span: DUMMY_SP }))
}
#[inline]
pub fn ast_create_expr_ident(id: &str) -> Box<Expr> {
  Box::new(Expr::Ident(ast_create_ident(id)))
}
#[inline]
pub fn ast_create_expr_member(obj: Box<Expr>, prop: MemberProp) -> Box<Expr> {
  Box::new(Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj,
    prop,
  }))
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
pub fn ast_create_expr_lit_string(v: String) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Atom::from(v),
    raw: None,
  })))
}
#[inline]
pub fn ast_create_expr_lit_bool(v: bool) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value: v,
  })))
}
#[inline]
pub fn ast_create_expr_call(callee: Box<Expr>, args: Vec<ExprOrSpread>) -> Box<Expr> {
  Box::new(Expr::Call(CallExpr {
    ctxt: SyntaxContext::empty(),
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
      ctxt: SyntaxContext::empty(),
      span: DUMMY_SP,
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(Ident::from("console"))),
        prop: MemberProp::Ident(IdentName::from("log")),
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

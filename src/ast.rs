use swc_core::{common::*, ecma::ast::*, ecma::atoms::*};

use crate::common::ImportId;

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
pub fn ast_create_expr_lit_str(v: &str) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Atom::from(v),
    raw: None,
  })))
}
pub fn ast_create_jinge_import(imports: Vec<ImportId>) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: imports
      .iter()
      .map(|i| {
        let sym = i.as_ref();
        let loc = format!("jinge${}$", sym);
        ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: Ident::from(loc.as_str()),
          imported: Some(ModuleExportName::Ident(Ident::from(sym))),
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

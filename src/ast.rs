use swc_core::{
  common::*,
  ecma::{ast::*, atoms::*},
};

use crate::common::{JINGE_EMPTY_STR, JINGE_HOST_IDENT};

// #[inline]
// pub fn ast_create_expr_new(callee: Box<Expr>, args: Option<Vec<ExprOrSpread>>) -> Box<Expr> {
//   Box::new(Expr::New(NewExpr {
//     span: DUMMY_SP,
//     ctxt: SyntaxContext::empty(),
//     callee,
//     args,
//     type_args: None,
//   }))
// }
// #[inline]
// pub fn ast_create_id_of_el(slot_level: usize) -> Ident {
//   if slot_level > 0 {
//     Ident::from(format!("{}${}$", JINGE_EL_IDENT.sym.as_str(), slot_level))
//   } else {
//     JINGE_EL_IDENT.clone()
//   }
// }
// #[inline]
// pub fn ast_create_id_of_container(slot_level: usize) -> Box<Expr> {
//   if slot_level == 0 {
//     ast_create_expr_this()
//   } else {
//     ast_create_expr_ident(ast_create_id_of_el(slot_level - 1))
//   }
// }
#[inline]
pub fn ast_create_id_of_container(is_root_container: bool) -> Box<Expr> {
  if is_root_container {
    ast_create_expr_this()
  } else {
    ast_create_expr_ident(JINGE_HOST_IDENT.clone())
  }
}
#[inline]
pub fn ast_create_stmt_decl_const(ident: Ident, init: Box<Expr>) -> Stmt {
  Stmt::Decl(Decl::Var(Box::new(VarDecl {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    kind: VarDeclKind::Const,
    declare: false,
    decls: vec![VarDeclarator {
      span: DUMMY_SP,
      definite: false,
      name: Pat::Ident(ident.into()),
      init: Some(init),
    }],
  })))
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
pub fn ast_create_expr_ident(id: Ident) -> Box<Expr> {
  Box::new(Expr::Ident(id))
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
pub fn ast_create_expr_lit_str(v: Atom) -> Box<Expr> {
  Box::new(Expr::Lit(Lit::Str(Str::from(v))))
}
#[inline]
pub fn ast_create_constructor(params: Vec<ParamOrTsParamProp>, stmts: Vec<Stmt>) -> Constructor {
  Constructor {
    span: DUMMY_SP,
    params,
    is_optional: false,
    key: PropName::Ident(IdentName::from(JINGE_EMPTY_STR.clone())),
    accessibility: None,
    ctxt: SyntaxContext::empty(),
    body: Some(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts,
    }),
  }
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
#[inline]
pub fn ast_create_expr_call_super(args: Vec<ExprOrSpread>) -> Box<Expr> {
  Box::new(Expr::Call(CallExpr {
    ctxt: SyntaxContext::empty(),
    span: DUMMY_SP,
    callee: Callee::Super(Super { span: DUMMY_SP }),
    args,
    type_args: None,
  }))
}
#[inline]
pub fn ast_create_expr_arrow_fn(params: Vec<Pat>, body: Box<BlockStmtOrExpr>) -> Box<Expr> {
  Box::new(Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    params: params,
    body,
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
  }))
}
#[inline]
pub fn ast_create_expr_assign_mem(obj: Box<Expr>, prop: Atom, value: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Assign(AssignExpr {
    span: DUMMY_SP,
    op: AssignOp::Assign,
    left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
      span: DUMMY_SP,
      obj,
      prop: MemberProp::Ident(IdentName::from(prop)),
    })),
    right: value,
  }))
}
// #[inline]
// pub fn ast_create_console_log() -> ModuleItem {
//   ModuleItem::Stmt(Stmt::Expr(ExprStmt {
//     span: DUMMY_SP,
//     expr: Box::new(Expr::Call(CallExpr {
//       ctxt: SyntaxContext::empty(),
//       span: DUMMY_SP,
//       callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
//         span: DUMMY_SP,
//         obj: Box::new(Expr::Ident(Ident::from("console"))),
//         prop: MemberProp::Ident(IdentName::from("log")),
//       }))),
//       args: vec![ExprOrSpread {
//         spread: None,
//         expr: Box::new(Expr::Lit(Lit::Str(Str {
//           span: DUMMY_SP,
//           value: Atom::from("hello"),
//           raw: None,
//         }))),
//       }],
//       type_args: None,
//     })),
//   }))
// }

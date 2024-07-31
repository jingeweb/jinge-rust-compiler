use swc_core::{
  atoms::Atom,
  common::{SyntaxContext, DUMMY_SP},
  ecma::ast::*,
};

use crate::{ast::*, common::*};

pub fn tpl_set_ref_code(r: Box<Expr>) -> Box<Expr> {
  let args = vec![
    ast_create_arg_expr(ast_create_expr_this()),
    ast_create_arg_expr(r),
    ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
  ];

  ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_SET_REF.local()), args)
}

pub fn tpl_push_el_code(root: bool, is_root_container: bool) -> Box<Expr> {
  let args = vec![ast_create_arg_expr(ast_create_expr_ident(
    JINGE_EL_IDENT.clone(),
  ))];
  Box::new(Expr::Call(CallExpr {
    ctxt: SyntaxContext::empty(),
    span: DUMMY_SP,
    callee: Callee::Expr(ast_create_expr_member(
      ast_create_expr_member(
        ast_create_id_of_container(is_root_container),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(if root {
            JINGE_IMPORT_ROOT_NODES.local()
          } else {
            JINGE_IMPORT_NON_ROOT_COMPONENT_NODES.local()
          }),
        }),
      ),
      MemberProp::Ident(IdentName::from("push")),
    )),
    args,
    type_args: None,
  }))
}

pub fn tpl_lit_obj(lit_arr: Vec<(IdentName, Box<Expr>)>) -> Box<Expr> {
  Box::new(Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: lit_arr
      .into_iter()
      .map(|(prop, value)| {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Str(Str::from(prop.sym)),
          value,
        })))
      })
      .collect(),
  }))
}

pub fn tpl_set_attribute(el: Box<Expr>, attr_name: Atom, attr_value: Box<Expr>) -> Box<Expr> {
  ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_SET_ATTRIBUTE.local()),
    vec![
      ast_create_arg_expr(el),
      ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(Str::from(attr_name))))),
      ast_create_arg_expr(attr_value),
    ],
  )
}

pub fn tpl_set_idl_attribute(el: Box<Expr>, attr_name: Atom, attr_value: Box<Expr>) -> Box<Expr> {
  Box::new(Expr::Assign(AssignExpr {
    span: DUMMY_SP,
    op: AssignOp::Assign,
    left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
      span: DUMMY_SP,
      obj: el,
      prop: MemberProp::Ident(IdentName::from(attr_name)),
    })),
    right: attr_value,
  }))
}

pub fn tpl_slot() -> Stmt {
  
}
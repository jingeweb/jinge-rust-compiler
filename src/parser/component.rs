use super::tpl::{
  tpl_lit_obj, tpl_push_el_code, tpl_set_ref_code, tpl_watch_and_set_component_attr,
};
use super::{Parent, TemplateParser};
use crate::ast::{
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
  ast_create_expr_member, ast_create_id_of_container, ast_create_stmt_decl_const,
};
use crate::common::*;
use swc_core::common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitWith;

impl TemplateParser {
  pub fn parse_component_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, true);
    let is_attrs_empty = attrs.const_props.is_empty() && attrs.watch_props.is_empty();
    self.push_context(Parent::Component, false);
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let children_context = self.pop_context();
    let root_container = self.context.root_container;

    let mut stmts: Vec<Stmt> = vec![];

    if !is_attrs_empty {
      stmts.push(ast_create_stmt_decl_const(
        JINGE_ATTR_IDENT.clone(),
        if !attrs.watch_props.is_empty() {
          ast_create_expr_call(
            ast_create_expr_ident(JINGE_IMPORT_VM.local()),
            vec![ast_create_arg_expr(tpl_lit_obj(attrs.const_props))],
          )
        } else {
          tpl_lit_obj(attrs.const_props)
        },
      ));
    }

    attrs
      .watch_props
      .into_iter()
      .for_each(|(attr_name, expr_result)| {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_set_component_attr(
            attr_name,
            expr_result,
            self.context.root_container,
          ),
        }));
      });

    let set_ref_code = attrs.ref_prop.take().map(|r| tpl_set_ref_code(r));
    let mut slots = children_context.slots;
    let mut args = vec![ast_create_arg_expr(ast_create_expr_member(
      ast_create_id_of_container(root_container),
      MemberProp::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
      }),
    ))];
    let has_named_slots = slots.len() > 1;
    if has_named_slots {
      assert!(slots[0].expressions.is_empty());
      let x: Vec<_> = slots
        .into_iter()
        .skip(1)
        .filter(|s| !s.expressions.is_empty()) // 跳过默认 DEFAULT_SLOT，一定是空的
        .map(|mut s| {
          let mut params = vec![Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone()))];
          params.append(&mut s.params);
          (
            IdentName::from(s.name),
            ast_create_expr_arrow_fn(
              params,
              Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: s.expressions.into_iter().map(|e| Some(e)).collect(),
              })))),
            ),
          )
        })
        .collect();
      args.push(ast_create_arg_expr(tpl_lit_obj(x)));
    } else {
      let mut default_slot = slots.pop().unwrap();
      if !default_slot.expressions.is_empty() {
        let mut params = vec![Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone()))];
        params.append(&mut default_slot.params);
        args.push(ast_create_arg_expr(ast_create_expr_arrow_fn(
          params,
          Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: default_slot
              .expressions
              .into_iter()
              .map(|e| Some(e))
              .collect(),
          })))),
        )))
      }
    }

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(if has_named_slots {
          JINGE_IMPORT_NEW_COM_SLOTS.local()
        } else {
          JINGE_IMPORT_NEW_COM_DEFAULT_SLOT.local()
        }),
        args,
      ),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), root_container),
    }));
    if let Some(c) = set_ref_code {
      stmts.push(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: c,
      }))
    }

    let mut render_fc_args = vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      ast_create_arg_expr(ast_create_expr_ident(Ident::from(tn.sym.clone()))),
    ];
    if !is_attrs_empty {
      render_fc_args.push(ast_create_arg_expr(ast_create_expr_ident(
        JINGE_ATTR_IDENT.clone(),
      )));
    }
    stmts.push(Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_RENDER_FC.local()),
        render_fc_args,
      )),
    }));
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
        spread: Some(DUMMY_SP),
        expr: ast_create_expr_call(
          ast_create_expr_arrow_fn(
            vec![],
            Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
              span: DUMMY_SP,
              ctxt: SyntaxContext::empty(),
              stmts,
            })),
          ),
          vec![],
        ),
      });
  }
}

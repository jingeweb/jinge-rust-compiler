use super::slot::get_slot_name_from_member_expr;
use super::tpl::{
  tpl_lit_obj, tpl_push_el_code, tpl_set_ref_code, tpl_watch_and_set_component_attr,
};
use super::{Parent, TemplateParser};
use crate::ast::*;
use crate::common::*;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitWith;

fn slot_to_expr(
  mut params: Vec<Pat>,
  mut stmts: Vec<Stmt>,
  expressions: Vec<ExprOrSpread>,
) -> Box<Expr> {
  let rtn_expr = Box::new(Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: expressions.into_iter().map(|e| Some(e)).collect(),
  }));
  if params.is_empty() {
    params.push(Pat::Ident(BindingIdent::from(JINGE_ATTR_IDENT.clone())));
    params.push(Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone())));
  }
  ast_create_expr_arrow_fn(
    params,
    Box::new(if stmts.is_empty() {
      BlockStmtOrExpr::Expr(rtn_expr)
    } else {
      stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(rtn_expr),
      }));
      BlockStmtOrExpr::BlockStmt(BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: stmts,
      })
    }),
  )
}

impl TemplateParser {
  fn parse_slot_pass_by(&mut self, n: &JSXElement) -> bool {
    if n.children.len() != 1 {
      return false;
    }
    let expr = &n.children[0];
    match expr {
      JSXElementChild::JSXExprContainer(e) => match &e.expr {
        JSXExpr::Expr(e) => match e.as_ref() {
          Expr::Member(mem_expr) => {
            if let Some(slot_name) = get_slot_name_from_member_expr(mem_expr, &self.props_arg) {
              self
                .context
                .slots
                .last_mut()
                .unwrap()
                .pass_by
                .replace(slot_name);
              return true;
            }
          }
          _ => (),
        },
        _ => (),
      },
      _ => (),
    }
    false
  }
  pub fn parse_component_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, true);
    // let is_attrs_empty = attrs.const_props.is_empty() && attrs.watch_props.is_empty();
    self.push_context(Parent::Component);

    // 如果是 <B>{props.children}</B> 这种写法，说明是透传插槽，可特别处理，直接传递。
    if !self.parse_slot_pass_by(n) {
      // 其它写法，比如 <B>hello: {props.children}</B> 这种需要转换成渲染模板。

      // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
      n.children.iter().for_each(|child| {
        child.visit_children_with(self);
      });
    }

    let children_context = self.pop_context();
    let host_ident = self.create_host_ident();

    let mut stmts: Vec<Stmt> = vec![];

    let mut const_props = attrs.const_props;
    let mut slots = children_context.slots;
    if !attrs.slot_props.is_empty() {
      slots.append(&mut attrs.slot_props);
    }

    slots.into_iter().enumerate().for_each(|(i, s)| {
      if s.expressions.is_empty() && s.pass_by.is_none() {
        return;
      }
      const_props.push((
        if i == 0 {
          // 第 0 个是默认 slot
          IdentName::from(JINGE_SLOT_DEFAULT.clone())
        } else {
          IdentName::from(format!("slot:{}", s.name))
        },
        if let Some(pass_by) = s.pass_by {
          pass_by
        } else {
          slot_to_expr(s.params, s.stmts, s.expressions)
        },
      ))
    });

    stmts.push(ast_create_stmt_decl_const(
      JINGE_ATTR_IDENT.clone(),
      if !attrs.watch_props.is_empty() {
        ast_create_expr_call(
          ast_create_expr_ident(JINGE_IMPORT_VM.local()),
          vec![ast_create_arg_expr(tpl_lit_obj(const_props))],
        )
      } else {
        tpl_lit_obj(const_props)
      },
    ));

    attrs
      .watch_props
      .into_iter()
      .for_each(|(attr_name, expr_result)| {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_set_component_attr(attr_name, expr_result, host_ident.clone()),
        }));
      });

    let set_ref_code = attrs
      .ref_prop
      .take()
      .map(|r| tpl_set_ref_code(r, host_ident.clone()));

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      Box::new(Expr::New(NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
        callee: ast_create_expr_ident(JINGE_IMPORT_COMPONENT_HOST.local()),
        args: Some(vec![ast_create_arg_expr(ast_create_expr_member(
          ast_create_expr_ident(host_ident.clone()),
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
          }),
        ))]),
        type_args: None,
      })),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), host_ident),
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
    if let Some(id) = attrs.spread_prop.take() {
      render_fc_args.push(ast_create_arg_expr(ast_create_expr_ident(id)));
    } else {
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

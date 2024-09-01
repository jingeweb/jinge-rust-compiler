use swc_common::{Spanned, SyntaxContext};
use swc_core::ecma::ast::*;
use swc_ecma_visit::VisitWith;

use crate::{ast::*, parser::*};

use super::{emit_error, TemplateParser};

impl TemplateParser {
  fn parse_html_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, false);
    self.push_context(
      if JINGE_SVG.eq(&tn.sym) {
        Parent::Svg
      } else {
        Parent::Html
      },
      self.context.root_container,
    );
    // println!("meet html {} {}", tn.sym.as_str(), self.context.slot_level);
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let root_container = self.context.root_container;
    let mut children_context = self.pop_context();
    // html 元素下不可能出现多个 slots。事实上，html 元素没有 slot 概念，只是用统一的数据结构保存子节点。
    assert_eq!(children_context.slots.len(), 1);
    let callee_ident = if self.context.is_parent_svg() || tn.sym.eq("svg") {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_ELE.local()
      }
    } else {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_ELE.local()
      }
    };
    let mut args = vec![ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(
      Str::from(tn.sym.clone()),
    ))))];
    let set_ref_code = attrs.ref_prop.take().map(|r| tpl_set_ref_code(r));
    let push_ele_code = if self.context.is_parent_component() {
      Some(tpl_push_el_code(true, root_container))
    } else {
      None
    };
    if !attrs.const_props.is_empty() {
      args.push(ast_create_arg_expr(tpl_lit_obj(attrs.const_props)));
    }
    if !children_context.slots[0].expressions.is_empty() {
      args.append(&mut children_context.slots[0].expressions);
    }

    let output = if set_ref_code.is_some()
      || push_ele_code.is_some()
      || !attrs.evt_props.is_empty()
      || !attrs.watch_props.is_empty()
    {
      let mut stmts: Vec<Stmt> = vec![ast_create_stmt_decl_const(
        JINGE_EL_IDENT.clone(),
        ast_create_expr_call(ast_create_expr_ident(callee_ident), args),
      )];
      attrs.evt_props.into_iter().for_each(|evt| {
        let mut args = vec![
          ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
          ast_create_arg_expr(ast_create_expr_lit_str(evt.event_name)),
          ast_create_arg_expr(evt.event_handler),
        ];
        if evt.capture {
          args.push(ast_create_arg_expr(ast_create_expr_lit_bool(true)));
        }
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_ADD_EVENT.local()), args),
        }))
      });
      attrs
        .watch_props
        .into_iter()
        .for_each(|(attr_name, watch_expr)| {
          stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: tpl_watch_and_set_html_attr(attr_name, watch_expr, self.context.root_container),
          }));
        });
      if let Some(c) = set_ref_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      if let Some(c) = push_ele_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      }));
      let body = Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
        ctxt: SyntaxContext::empty(),
        span: DUMMY_SP,
        stmts,
      }));
      let callee = Box::new(Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(Expr::Arrow(ArrowExpr {
          ctxt: SyntaxContext::empty(),
          span: DUMMY_SP,
          params: vec![],
          body,
          is_async: false,
          is_generator: false,
          type_params: None,
          return_type: None,
        })),
      }));
      ast_create_expr_call(callee, vec![])
    } else {
      ast_create_expr_call(ast_create_expr_ident(callee_ident), args)
    };
    // 当前 html 元素添加到父亲的最顶部 Slot 中。最顶部 Slot 可能是默认 Slot(比如父亲也是 html 元素则也是存放在默认 Slot)，也可能是命名 Slot(只可能出现在父亲是组件的情况)
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
        spread: None,
        expr: output,
      });
  }

  pub fn parse_jsx_element(&mut self, n: &JSXElement) {
    let JSXElementName::Ident(tn) = &n.opening.name else {
      emit_error(
        n.opening.name.span(),
        "未知的 JSX 格式，opening.name 未找到",
      );
      return;
    };
    // let tag = tn.as_ref();
    // println!("visit jsx ele: {}", tn.as_ref());
    match tn.as_ref().chars().next() {
      Some(c) if c.is_ascii_uppercase() => {
        self.parse_component_element(tn, n);
      }
      Some(c) if c.is_ascii_lowercase() => {
        self.parse_html_element(tn, n);
      }
      _ => {
        emit_error(
          tn.span(),
          "不支持的 Tag。合法 Tag 为：大写字母打头为 Component 组件，小写字母打头为 html 元素。",
        );
        return;
      }
    }
  }
}

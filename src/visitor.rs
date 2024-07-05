use std::borrow::Borrow;
use std::ops::Deref;

use swc_core::common::{Span, Spanned};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitMut};
use swc_core::plugin::errors::HANDLER;

use crate::config::Config;
use swc_core::ecma::visit::VisitMutWith;

pub struct TransformVisitor {
  pub cwd: String,
  pub filename: String,
  pub config: Config,
}

fn emit_error(sp: Span, msg: &str) {
  HANDLER.with(|h| {
    h.struct_span_err(sp, msg).emit();
  });
}
impl VisitMut for TransformVisitor {
  // Implement necessary visit_mut_* methods for actual custom transform.
  // A comprehensive list of possible visitor methods can be found here:
  // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html

  // fn visit_mut_call_expr(&mut self, n: &mut swc_core::ecma::ast::CallExpr) {}

  // fn visit_mut_module(&mut self, n: &mut Module) {
  //   n.visit_mut_children_with(self);
  //   // transform_server_action(self, n);
  // }
  fn visit_mut_class_decl(&mut self, n: &mut ClassDecl) {
    if !matches!(&n.class.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
    {
      n.visit_mut_children_with(self);
      return;
    }
    println!("got {}", n.ident.sym.as_str());

    let render = n.class.body.iter().find(|it| matches!(it, ClassMember::Method(it) if matches!(&it.key, PropName::Ident(it) if it.sym.as_str() == "render")));
    let Some(render) = render else {
      emit_error(n.ident.span(), "组件缺失 render() 函数");
      return;
    };
    let render_fn = match render {
      ClassMember::Method(r) => r.function.as_ref(),
      _ => unreachable!(),
    };
    let Some(expr) = render_fn.body.as_ref().and_then(|body| {
      if let Some(Stmt::Return(stmt)) = body.stmts.last() {
        stmt.arg.as_ref()
      } else {
        None
      }
    }) else {
      // 如果最后一条语句不是 return JSX，则不把 render() 函数当成需要处理的渲染模板。
      return;
    };
    match expr.as_ref() {
      Expr::Paren(expr) => {
        let expr = expr.expr.as_ref();
        match expr {
          Expr::JSXFragment(expr) => {}
          Expr::JSXElement(expr) => {
            let mut vt = JSXVisitor {};
            vt.visit_jsx_element(expr.as_ref());
          }
          _ => return,
        }
      }
      Expr::JSXElement(expr) => {
        let mut vt = JSXVisitor {};
        vt.visit_jsx_element(expr.as_ref());
      }
      Expr::JSXFragment(expr) => {}
      _ => return,
    };

    println!("got render() {:?}", render);
  }
}

struct JSXVisitor {}

impl Visit for JSXVisitor {}

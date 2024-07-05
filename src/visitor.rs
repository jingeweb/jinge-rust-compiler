use std::borrow::Borrow;
use std::ops::Deref;

use swc_core::common::{Span, Spanned};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitMut, VisitWith};
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

  fn visit_mut_module(&mut self, n: &mut Module) {
    n.visit_mut_children_with(self);

    n.body.iter_mut().for_each(|item| match item {
      ModuleItem::ModuleDecl(decl) => match decl {
        ModuleDecl::ExportDecl(decl) => match &mut decl.decl {
          Decl::Class(cls) => visit_mut_class_decl(cls),
          Decl::Var(decl) => decl.as_mut().decls.iter_mut().for_each(|decl| {
            if let Some(x) = &mut decl.init {
              match x.as_mut() {
                Expr::Class(cls) => visit_mut_class_expr(cls),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        ModuleDecl::ExportDefaultDecl(decl) => match &mut decl.decl {
          DefaultDecl::Class(cls) => visit_mut_class_expr(cls),
          _ => (),
        },
        _ => (),
      },
      ModuleItem::Stmt(stmt) => match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Class(decl) => visit_mut_class_decl(decl),
          Decl::Var(decl) => decl.decls.iter_mut().for_each(|decl| {
            if let Some(decl) = decl.init.as_mut() {
              match decl.as_mut() {
                Expr::Class(cls) => visit_mut_class_expr(cls),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        _ => (),
      },
    })
  }
}

fn visit_mut_class_expr(n: &mut ClassExpr) {
  if !matches!(&n.class.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
  {
    return;
  }
  visit_class(n.ident.as_ref(), &n.class);
}
fn visit_mut_class_decl(n: &mut ClassDecl) {
  if !matches!(&n.class.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
  {
    return;
  }
  visit_class(Some(&n.ident), &n.class);
  println!("got {}", n.ident.sym.as_str());
}

fn visit_class(ident: Option<&Ident>, class: &Class) {
  let render = class.body.iter().find(|it| matches!(it, ClassMember::Method(it) if matches!(&it.key, PropName::Ident(it) if it.sym.as_str() == "render")));
  let Some(render) = render else {
    let span = if let Some(ident) = ident {
      ident.span()
    } else {
      class.span()
    };
    emit_error(span, "组件缺失 render() 函数");
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
  let visitor = match expr.as_ref() {
    Expr::Paren(expr) => visit_expr(expr.expr.as_ref()),
    _ => visit_expr(expr),
  };
  if let Some(output) = visitor.output.as_ref() {
    println!("changed.")
  }
}
fn visit_expr(expr: &Expr) -> JSXVisitor {
  let mut visitor = JSXVisitor::new();
  if match expr {
    Expr::Lit(_) => true,
    Expr::JSXElement(_) => true,
    Expr::JSXFragment(_) => true,
    _ => false,
  } {
    visitor.visit_expr(expr);
  }
  visitor
}

struct JSXVisitor {
  output: Option<String>,
}

impl JSXVisitor {
  fn new() -> Self {
    println!("New JSXVisitor");
    Self { output: None }
  }
}

impl Visit for JSXVisitor {
  fn visit_number(&mut self, n: &Number) {
    println!("{}", n.value);
  }
  fn visit_str(&mut self, n: &Str) {
    println!("{}", n.value);
  }
  fn visit_lit(&mut self, n: &Lit) {
    n.visit_children_with(self)
  }
}

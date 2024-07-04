use std::ops::Deref;

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;

use crate::config::Config;
use swc_core::ecma::visit::VisitMutWith;

pub struct TransformVisitor {
  pub cwd: String,
  pub filename: String,
  pub config: Config,
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

    let render = n.class.body.iter().find(|it| matches!(it, ClassMember::Method(it) if matches!(&it.key, PropName::Ident(it) if it.sym.as_str() == "render"))).unwrap_or_else(|| {
      panic!("Component {} must have render() function", n.ident.sym.as_str());
    });

    println!("got render()");
  }
}

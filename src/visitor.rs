use swc_core::ecma::ast::Module;
use swc_core::ecma::visit::VisitMut;

use crate::config::Config;
use crate::intl::visit_mut_call_expr;
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

  fn visit_mut_call_expr(&mut self, n: &mut swc_core::ecma::ast::CallExpr) {
    visit_mut_call_expr(self, n);
  }

  fn visit_mut_module(&mut self, n: &mut Module) {
    n.visit_mut_children_with(self);
    // transform_server_action(self, n);
  }
}

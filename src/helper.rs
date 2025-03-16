use swc_core::ecma::ast::{Callee, Expr};

use crate::common::JINGE_T;

pub fn has_jsx(expr: &Expr) -> bool {
  match expr {
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(e) => {
      return has_jsx(&e.alt) || has_jsx(&e.cons);
    }
    Expr::Bin(e) => return has_jsx(&e.left) || has_jsx(&e.right),
    Expr::Paren(e) => return has_jsx(&e.expr),
    Expr::Call(e) => match &e.callee {
      Callee::Expr(e) => match e.as_ref() {
        Expr::Ident(id) if JINGE_T.eq(&id.sym) => true,
        _ => false,
      },
      _ => false,
    },
    _ => {
      return false;
    }
  }
}

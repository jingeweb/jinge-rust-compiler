use swc_core::ecma::ast::Expr;

pub fn has_jsx(expr: &Expr) -> bool {
  match expr {
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(e) => {
      return has_jsx(&e.alt) || has_jsx(&e.cons);
    }
    Expr::Bin(e) => return has_jsx(&e.left) || has_jsx(&e.right),
    Expr::Paren(e) => return has_jsx(&e.expr),
    _ => {
      return false;
    }
  }
}

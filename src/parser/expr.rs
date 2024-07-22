use swc_core::{
  atoms::Atom,
  common::Spanned,
  ecma::{ast::*, visit::Visit},
};

use crate::common::emit_error;

pub struct AttrExpr {
  pub name: Atom,
  pub is_const: bool,
}

pub fn parse_expr_attr(name: Atom, val: &Expr) -> AttrExpr {
  let mut parser = ExprAttrParser::new();
  parser.visit_expr(val);
  AttrExpr {
    name,
    is_const: parser.is_const,
  }
}

struct ExprAttrParser {
  is_const: bool,
}

impl ExprAttrParser {
  pub fn new() -> Self {
    Self { is_const: false }
  }
}

impl Visit for ExprAttrParser {}

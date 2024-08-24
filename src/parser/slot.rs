use swc_core::{
  atoms::Atom,
  ecma::ast::{CallExpr, Callee, Expr, MemberProp},
};

use super::TemplateParser;

lazy_static::lazy_static! {
  static ref SLOTS: Atom = Atom::from("slots");
}

enum Slot {
  None,
  Default,
  Named(Atom),
}
fn get_slot(expr: &CallExpr) -> Slot {
  let Callee::Expr(expr) = &expr.callee else {
    return Slot::None;
  };
  let Expr::Member(expr) = expr.as_ref() else {
    return Slot::None;
  };
  match expr.obj.as_ref() {
    Expr::This(_) => {
      let MemberProp::Ident(prop) = &expr.prop else {
        return Slot::None;
      };
      if prop.sym.eq(&SLOTS.clone()) {
        Slot::Default
      } else {
        Slot::None
      }
    }
    Expr::Member(expr2) => match expr2.obj.as_ref() {
      Expr::This(_) => {
        let MemberProp::Ident(prop) = &expr2.prop else {
          return Slot::None;
        };
        if !prop.sym.eq(&SLOTS.clone()) {
          return Slot::None;
        }
        let MemberProp::Ident(prop) = &expr.prop else {
          return Slot::None;
        };
        Slot::Named(prop.sym.clone())
      }
      _ => Slot::None,
    },
    _ => Slot::None,
  }
}
impl TemplateParser {
  pub fn parse_slot_call_expr(&mut self, expr: &CallExpr) -> bool {
    match get_slot(expr) {
      Slot::None => false,
      Slot::Default => true,
      Slot::Named(n) => true,
    }
  }
}

use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::ast::*;

use super::{
  expr::{ExprParseResult, ExprVisitor},
  tpl::{tpl_render_const_text, tpl_render_expr_text},
  TemplateParser, JINGE_V_IDENT,
};

impl TemplateParser {
  pub fn parse_cond_expr(&mut self, expr: &CondExpr) {
    let expr_result = ExprVisitor::new().parse(expr.test.as_ref());

    if matches!(expr.alt.as_ref(), Expr::Lit(_)) && matches!(expr.cons.as_ref(), Expr::Lit(_)) {
      if matches!(expr_result, ExprParseResult::None) {
        self.push_expression(tpl_render_const_text(
          Box::new(Expr::Cond(expr.clone())),
          self.context.is_parent_component(),
          self.context.root_container,
        ));
      } else {
        self.push_expression(tpl_render_expr_text(
          expr_result,
          Box::new(Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: ast_create_expr_ident(JINGE_V_IDENT.clone()),
            cons: expr.cons.clone(),
            alt: expr.alt.clone(),
          })),
          self.context.is_parent_component(),
          self.context.root_container,
        ));
      }
      return; // important to return !!
    }
  }
}

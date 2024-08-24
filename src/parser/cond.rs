use swc_common::Spanned;
use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::ast::*;

use super::{
  expr::{ExprParseResult, ExprVisitor},
  tpl::{tpl_render_const_text, tpl_render_expr_text},
  TemplateParser, JINGE_IMPORT_IF, JINGE_V_IDENT,
};

lazy_static::lazy_static! {
  static ref EXPECT: IdentName = IdentName::from("expect");
  static ref TRUE: IdentName = IdentName::from("true");
  static ref FALSE: IdentName = IdentName::from("false");
}

/// 将形如 `test ? cons : alt` 的二元条件表达式，转换为 `If` 组件： `<If expect={test}>{{ true: cons, false: alt }}</If>`
fn gen_if_component(expr: &CondExpr) -> JSXElement {
  JSXElement {
    span: expr.span(),
    opening: JSXOpeningElement {
      name: JSXElementName::Ident(JINGE_IMPORT_IF.local()),
      span: expr.cons.span(),
      attrs: vec![JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: expr.test.span(),
        name: JSXAttrName::Ident(EXPECT.clone()),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: expr.test.span(),
          expr: JSXExpr::Expr(expr.test.clone()),
        })),
      })],
      self_closing: false,
      type_args: None,
    },
    children: vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
      span: expr.alt.span(),
      expr: JSXExpr::Expr(Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(TRUE.clone()),
            value: expr.cons.clone(),
          }))),
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(FALSE.clone()),
            value: expr.alt.clone(),
          }))),
        ],
      }))),
    })],
    closing: Some(JSXClosingElement {
      name: JSXElementName::Ident(JINGE_IMPORT_IF.local()),
      span: DUMMY_SP,
    }),
  }
}

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

    /* 如果是复杂的 ? : 表达式，比如 `this.submitting ? <p>Submitting</p> : <span>SUBMIT</span>`，
    * 转换为 `If` 组件：```<If expect={this.submitting}>{{
         true: <p>Submitting</p>,
         false: <span>SUBMIT</p>
       }}</If>```
    */
    let if_component = gen_if_component(expr);
    self.parse_component_element(&JINGE_IMPORT_IF.local(), &if_component);
  }
}

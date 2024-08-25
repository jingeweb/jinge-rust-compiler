use swc_common::Spanned;
use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::ast::*;

use super::{
  expr::{ExprParseResult, ExprVisitor},
  tpl::{tpl_render_const_text, tpl_render_expr_text},
  TemplateParser, JINGE_EMPTY_STR, JINGE_IMPORT_IF, JINGE_UNDEFINED, JINGE_V_IDENT,
};

lazy_static::lazy_static! {
  static ref EXPECT: IdentName = IdentName::from("expect");
  static ref TRUE: IdentName = IdentName::from("true");
  static ref FALSE: IdentName = IdentName::from("false");
}

/// 将形如 `test ? cons : alt` 的二元条件表达式，转换为 `If` 组件： `<If expect={test}>{{ true: cons, false: alt }}</If>`
fn gen_if_component(
  expr: &CondExpr,
  is_alt_null_undef: bool,
  is_cons_null_undef: bool,
) -> JSXElement {
  let mut slots = Vec::with_capacity(if is_alt_null_undef || is_cons_null_undef {
    1
  } else {
    2
  });
  println!("{}, {}", is_alt_null_undef, is_cons_null_undef);
  if !is_cons_null_undef {
    slots.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(TRUE.clone()),
      value: expr.cons.clone(),
    }))));
  }
  if !is_alt_null_undef {
    slots.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(FALSE.clone()),
      value: expr.alt.clone(),
    }))));
  }
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
        props: slots,
      }))),
    })],
    closing: Some(JSXClosingElement {
      name: JSXElementName::Ident(JINGE_IMPORT_IF.local()),
      span: DUMMY_SP,
    }),
  }
}

/// 判定表达式是否是 null 或者 undefined。注意 null 是 Lit::Null，但 undefined 是 Ident 类型。
fn is_null_undef(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(id) => JINGE_UNDEFINED.eq(&id.sym),
    _ => false,
  }
}

impl TemplateParser {
  /// 为兼容 react 的 `test ? alt : cons` 写法，将条件表达式转成 <If> 组件：
  /// ```tsx
  /// <If expect={test}>{{ true: alt, false: cons }}</If>
  /// ```
  ///
  /// 如果 alt 和 cons 表达式都是常量表达式，比如常见的 `this.submitting ? "提交中..." : "提交"`，
  /// 则转换为更轻量的 watch & render-text-const 写法（参看生成的代码）
  ///
  /// 需要注意的是，jinge 框架对于 null/undefined/false 值会输出 JSON.stringify 后的文本，即不会像 react 框架那样直接忽略；
  /// 但为了尽可能兼容 react 的二元条件表达式的写法，对于在条件表达式中的常量 null/undefined，会被渲染忽略，因为业务场景里这样书写一定是需要忽略。
  pub fn parse_cond_expr(&mut self, expr: &CondExpr) {
    let expr_result = ExprVisitor::new().parse(expr.test.as_ref());
    let is_alt_null_undef = is_null_undef(expr.alt.as_ref());
    let is_cons_null_undef = is_null_undef(expr.cons.as_ref());
    if is_alt_null_undef && is_cons_null_undef {
      // 如果 alt 和 cons 表达式都是 null/undefined 常量，则这个表达式没有渲染意义，直接忽略。
      return;
    }

    // 如果 alt 和 cons 都是常量，则转成轻量 watch & render 写法。
    // 注意 undefined 不是 Lit 类型，是 Ident 类型。
    if (matches!(expr.alt.as_ref(), Expr::Lit(_)) || is_alt_null_undef)
      && (matches!(expr.cons.as_ref(), Expr::Lit(_)) || is_cons_null_undef)
    {
      let alt = if is_alt_null_undef {
        Box::new(Expr::Lit(Lit::Str(Str::from(JINGE_EMPTY_STR.clone()))))
      } else {
        expr.alt.clone()
      };
      let cons = if is_cons_null_undef {
        Box::new(Expr::Lit(Lit::Str(Str::from(JINGE_EMPTY_STR.clone()))))
      } else {
        expr.cons.clone()
      };
      if matches!(expr_result, ExprParseResult::None) {
        self.push_expression(tpl_render_const_text(
          Box::new(Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: expr.test.clone(),
            alt,
            cons,
          })),
          self.context.is_parent_component(),
          self.context.root_container,
        ));
      } else {
        self.push_expression(tpl_render_expr_text(
          expr_result,
          Box::new(Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: ast_create_expr_ident(JINGE_V_IDENT.clone()),
            alt,
            cons,
          })),
          self.context.is_parent_component(),
          self.context.root_container,
        ));
      }
      return; // important to return !!
    }

    // 如果是 alt 和 cons 是非常量的表达式，比如 `this.submitting ? <p>Submitting</p> : <span>SUBMIT</span>`，
    // 转换为 `If` 组件：```<If expect={this.submitting}>{{true: <p>Submitting</p>, false: <span>SUBMIT</p> }}</If>```
    let if_component = gen_if_component(expr, is_alt_null_undef, is_cons_null_undef);
    self.parse_component_element(&JINGE_IMPORT_IF.local(), &if_component);
  }
}

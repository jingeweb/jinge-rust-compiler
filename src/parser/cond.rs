use swc_common::Spanned;
use swc_core::{common::DUMMY_SP, ecma::ast::*};

use crate::{
  common::{JINGE_IMPORT_IF, JINGE_SLOT, JINGE_UNDEFINED},
  helper::has_jsx,
};

use super::TemplateParser;

lazy_static::lazy_static! {
  static ref EXPECT: IdentName = IdentName::from("expect");

  static ref ELSE: IdentName = IdentName::from("else");
}

/// 将形如 `test ? cons : alt` 的二元条件表达式，转换为 `If` 组件： `<If expect={test} slot:true={cons} slot:false={alt} />`
fn gen_if_component(
  test: Box<Expr>,
  alt: Option<&Box<Expr>>,
  cons: Option<&Box<Expr>>,
) -> JSXElement {
  let mut attrs = Vec::with_capacity(if alt.is_some() { 2 } else { 1 });
  attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: test.span(),
    name: JSXAttrName::Ident(EXPECT.clone()),
    value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: test.span(),
      expr: JSXExpr::Expr(test),
    })),
  }));

  if let Some(alt) = alt {
    attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::JSXNamespacedName(JSXNamespacedName {
        span: DUMMY_SP,
        ns: IdentName::from(JINGE_SLOT.clone()),
        name: IdentName::from(ELSE.clone()),
      }),
      value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(alt.clone()),
      })),
    }));
  }

  JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      name: JSXElementName::Ident(JINGE_IMPORT_IF.local()),
      span: cons.span(),
      attrs,
      self_closing: cons.is_none(),
      type_args: None,
    },
    children: if cons.is_some() {
      vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(cons.unwrap().clone()),
      })]
    } else {
      vec![]
    },
    closing: cons.map(|_| JSXClosingElement {
      name: JSXElementName::Ident(JINGE_IMPORT_IF.local()),
      span: DUMMY_SP,
    }),
  }
}

/// 判定表达式是否是 null 或者 undefined。注意 null 是 Lit::Null，但 undefined 是 Ident 类型。
#[inline]
fn is_null_undef(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(id) => JINGE_UNDEFINED.eq(&id.sym),
    _ => false,
  }
}

impl TemplateParser {
  /// 为兼容 react 的 `test ? cons : alt` 写法，将条件表达式转成 <If> 组件：
  /// ```tsx
  /// <If expect={test} slot:false={alt}>{cons}</If>
  /// ```
  ///
  /// 如果 alt 和 cons 表达式都是常量表达式，比如常见的 `this.submitting ? "提交中..." : "提交"`，
  /// 则转换为更轻量的 watch & render-text-const 写法（参看生成的代码）
  ///
  /// 需要注意的是，jinge 框架对于 null/undefined/false 值会输出 JSON.stringify 后的文本，即不会像 react 框架那样直接忽略；
  /// 但为了尽可能兼容 react 的二元条件表达式的写法，对于在条件表达式中的常量 null/undefined，会被渲染忽略，因为业务场景里这样书写一定是需要忽略。
  pub fn parse_cond_expr(&mut self, expr: &CondExpr) -> bool {
    if self.parse_cond_slot(expr) {
      return true;
    }
    if !has_jsx(&expr.alt) && !has_jsx(&expr.cons) {
      // 如果条件表达式两边都不是 jsx 元素，则返回  false，进行后续的 parse_expr
      return false;
    }
    let is_alt_null_undef = is_null_undef(expr.alt.as_ref());
    let is_cons_null_undef = is_null_undef(expr.cons.as_ref());
    if is_alt_null_undef && is_cons_null_undef {
      // 如果 alt 和 cons 表达式都是 null/undefined 常量，则这个表达式没有渲染意义，直接忽略。
      return true;
    }

    // 以下注释代码是废弃但暂时不删除的逻辑。新逻辑是，当左右都不是 jsx 时直接返回 false，使用 parse_expr 处理。
    // // 如果 alt 和 cons 都是常量，则转成轻量 watch & render 写法。
    // // 注意 undefined 不是 Lit 类型，是 Ident 类型。
    // if (matches!(expr.alt.as_ref(), Expr::Lit(_)) || is_alt_null_undef)
    //   && (matches!(expr.cons.as_ref(), Expr::Lit(_)) || is_cons_null_undef)
    // {
    //   let alt = if is_alt_null_undef {
    //     Box::new(Expr::Lit(Lit::Str(Str::from(JINGE_EMPTY_STR.clone()))))
    //   } else {
    //     expr.alt.clone()
    //   };
    //   let cons = if is_cons_null_undef {
    //     Box::new(Expr::Lit(Lit::Str(Str::from(JINGE_EMPTY_STR.clone()))))
    //   } else {
    //     expr.cons.clone()
    //   };
    //   let expr_result = ExprVisitor::new().parse(expr.test.as_ref());
    //   if matches!(expr_result, ExprParseResult::None) {
    //     self.push_expression(tpl_render_const_text(
    //       Box::new(Expr::Cond(CondExpr {
    //         span: DUMMY_SP,
    //         test: expr.test.clone(),
    //         alt,
    //         cons,
    //       })),
    //       self.context.is_parent_component(),
    //       self.context.root_container,
    //     ));
    //   } else {
    //     self.push_expression(tpl_render_expr_text(
    //       expr_result,
    //       Box::new(Expr::Cond(CondExpr {
    //         span: DUMMY_SP,
    //         test: ast_create_expr_ident(JINGE_V_IDENT.clone()),
    //         alt,
    //         cons,
    //       })),
    //       self.context.is_parent_component(),
    //       self.context.root_container,
    //     ));
    //   }
    //   return true; // important to return !!
    // }

    // 如果是 alt 和 cons 是非常量的表达式，比如 `this.submitting ? <p>Submitting</p> : <span>SUBMIT</span>`，
    // 转换为 `If` 组件：```<If expect={this.submitting}>{{true: <p>Submitting</p>, false: <span>SUBMIT</p> }}</If>```
    let if_component = gen_if_component(
      expr.test.clone(),
      if is_alt_null_undef {
        None
      } else {
        Some(&expr.alt)
      },
      if is_cons_null_undef {
        None
      } else {
        Some(&expr.cons)
      },
    );
    self.parse_component_element(&JINGE_IMPORT_IF.local(), &if_component);

    true
  }

  pub fn parse_logic_and_expr(&mut self, expr: &BinExpr) -> bool {
    if self.parse_logic_and_slot(expr) {
      return true;
    }
    if !has_jsx(&expr.right) {
      return false; // 返回 false，使用 parse_expr 处理。
    }

    let if_component = gen_if_component(expr.left.clone(), None, Some(&expr.right));
    self.parse_component_element(&JINGE_IMPORT_IF.local(), &if_component);

    true
  }

  pub fn parse_nullish_coalescing_expr(&mut self, expr: &BinExpr) -> bool {
    if self.parse_nullish_coalescing_slot(expr) {
      return true;
    }
    if !has_jsx(&expr.right) {
      return false; // 返回 false，使用 parse_expr 处理。
    }
    let test = Box::new(Expr::Bin(BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::EqEq,
      left: expr.left.clone(),
      right: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
    }));
    let if_component = gen_if_component(test, Some(&expr.left), Some(&expr.right));
    self.parse_component_element(&JINGE_IMPORT_IF.local(), &if_component);

    true
  }
}

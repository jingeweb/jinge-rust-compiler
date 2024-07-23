use crate::common::emit_error;
use crate::parser::TemplateParser;
use swc_core::common::Spanned;
use swc_core::ecma::ast::*;

use super::expr::{parse_expr_attr, AttrExpr};

pub struct AttrEvt {
  pub event_name: String,
  pub event_handler: Box<Expr>,
  pub capture: bool,
}
pub struct AttrStore {
  /// ref 属性，例如 `<div ref="some"></div>`
  pub ref_prop: Option<Lit>,
  /// 事件属性，例如 `<div onClick={(evt) => {}}></div>`
  pub evt_props: Vec<AttrEvt>,
  /// 不需要 watch 监听的表达式属性，例如 `<div a={45 + "hello"} b={_someVar.o} c="hello" d={true} disabled ></div>`
  pub const_props: Vec<(IdentName, Box<Expr>)>,
  /// 需要 watch 监听的表达式属性，例如 `<div a={this.some}></div>`
  pub watch_props: Vec<AttrExpr>,
}

impl TemplateParser {
  pub fn parse_attrs(&mut self, n: &JSXElement) -> AttrStore {
    let mut attrs = AttrStore {
      ref_prop: None,
      evt_props: vec![],
      const_props: vec![],
      watch_props: vec![],
    };

    n.opening.attrs.iter().for_each(|attr| match attr {
      JSXAttrOrSpread::SpreadElement(s) => {
        emit_error(s.span(), "暂不支持 ... 属性");
      }
      JSXAttrOrSpread::JSXAttr(attr) => {
        let JSXAttrName::Ident(an) = &attr.name else {
          return;
        };
        let name = &an.sym;
        if name == "ref" {
          if attrs.ref_prop.is_some() {
            emit_error(attr.span(), "不能重复指定 ref");
            return;
          }
          let Some(JSXAttrValue::Lit(val)) = &attr.value else {
            emit_error(attr.span(), "ref 属性值只能是字符串");
            return;
          };
          if !matches!(val, Lit::Str(_)) {
            emit_error(attr.span(), "ref 属性值只能是字符串");
            return;
          }

          attrs.ref_prop.replace(val.clone());
        } else if name.starts_with("on")
          && matches!(name.chars().nth(2), Some(c) if c >= 'A' && c <= 'Z')
        {
          let Some(JSXAttrValue::JSXExprContainer(val)) = &attr.value else {
            emit_error(attr.span(), "事件属性的属性值必须是箭头函数");
            return;
          };
          let JSXExpr::Expr(val) = &val.expr else {
            emit_error(attr.span(), "事件属性的属性值必须是箭头函数");
            return;
          };
          if !matches!(val.as_ref(), Expr::Arrow(_)) {
            emit_error(attr.span(), "事件属性的属性值必须是箭头函数");
            return;
          };
          let mut event_name = &name[2..];
          let mut capture = false;
          if event_name.ends_with("Capture") {
            event_name = &event_name[..event_name.len() - 7];
            capture = true;
          }
          attrs.evt_props.push(AttrEvt {
            event_name: event_name.to_lowercase(),
            event_handler: val.clone(),
            capture,
          })
        } else {
          if let Some(val) = &attr.value {
            match val {
              JSXAttrValue::Lit(val) => {
                attrs
                  .const_props
                  .push((an.clone(), Box::new(Expr::Lit(val.clone()))));
              }
              JSXAttrValue::JSXExprContainer(val) => match &val.expr {
                JSXExpr::JSXEmptyExpr(_) => (),
                JSXExpr::Expr(expr) => match expr.as_ref() {
                  Expr::JSXElement(_)
                  | Expr::JSXEmpty(_)
                  | Expr::JSXFragment(_)
                  | Expr::JSXMember(_)
                  | Expr::JSXNamespacedName(_) => {
                    emit_error(val.expr.span(), "不支持 JSX 元素作为属性值");
                  }
                  Expr::Lit(val) => {
                    attrs
                      .const_props
                      .push((an.clone(), Box::new(Expr::Lit(val.clone()))));
                  }
                  Expr::Fn(_) | Expr::Arrow(_) => emit_error(
                    attr.name.span(),
                    "不支持函数作为属性值。如果是想传递事件，请使用 on 打头的属性名，例如 onClick",
                  ),
                  _ => {
                    if let AttrExpr::Watch(expr) = parse_expr_attr(expr.as_ref()) {
                      //
                    } else {
                      attrs.const_props.push((an.clone(), expr.clone()));
                    }
                  }
                },
              },
              _ => emit_error(val.span(), "不支持该类型的属性值。"),
            }
          } else {
            // bool attribute
            attrs
              .const_props
              .push((an.clone(), Box::new(Expr::Lit(Lit::Bool(Bool::from(true))))));
          }
        }
      }
    });

    attrs
  }
}

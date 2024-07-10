use swc_core::atoms::Atom;
use swc_core::common::Spanned;
use swc_core::ecma::ast::{Bool, Expr, Ident, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElement, JSXExpr, Lit};
use crate::common::emit_error;
use crate::parser::TemplateParser;

pub struct AttrStore {
  pub ref_mark: Option<Atom>,
  pub lit_props: Vec<(Ident, Lit)>
}

impl TemplateParser {
  pub fn parse_attrs(&mut self, tag: &Ident, n: &JSXElement) -> AttrStore {
    let mut attrs = AttrStore {
      ref_mark: None,
      lit_props: vec![]
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
          if attrs.ref_mark.is_some() {
            emit_error(attr.span(), "不能重复指定 ref");
            return;
          }
          let Some(JSXAttrValue::Lit(Lit::Str(val))) = &attr.value else {
            emit_error(attr.span(), "ref 属性值只能是字符串");
            return;
          };
          attrs.ref_mark.replace(val.value.clone());
        } else if name.starts_with("on")
          && matches!(name.chars().nth(2), Some(c) if c >= 'A' && c <= 'Z')
        {
          // html event
        } else {
          if let Some(val) = &attr.value {
            match val {
              JSXAttrValue::Lit(val) => {
                attrs.lit_props.push((an.clone(), val.clone()));
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
                    attrs.lit_props.push((an.clone(), val.clone()));
                  }
                  Expr::Fn(_) | Expr::Arrow(_) => {
                    emit_error(attr.name.span(), "不支持函数作为属性值。如果是想传递事件，请使用 on 打头的属性名，例如 onClick")
                  }
                  _ => {
                    // expr attribute
                  }
                },
              },
              _ => emit_error(val.span(), "不支持该类型的属性值。"),
            }
          } else {
            // bool attribute
            attrs.lit_props.push((an.clone(), Lit::Bool(Bool::from(true))));
          }
        }
      }
    });

    attrs
  }
}
use std::rc::Rc;

use crate::common::emit_error;
use crate::parser::TemplateParser;
use hashbrown::HashSet;
use swc_core::{atoms::Atom, common::Spanned};
use swc_core::ecma::ast::*;

use super::expr::{ExprParseResult, ExprVisitor};
use super::{JINGE_CHILDREN, JINGE_CLASS, JINGE_CLASSNAME, JINGE_LOOP_KEY, JINGE_REF, JINGE_SLOTS};

pub struct AttrEvt {
  pub event_name: Atom,
  pub event_handler: Box<Expr>,
  pub capture: bool,
}
pub struct AttrStore {
  /// ref 属性，例如 `<div ref="some"></div>`
  pub ref_prop: Option<Box<Expr>>,
  /// 事件属性，例如 `<div onClick={(evt) => {}}></div>`
  pub evt_props: Vec<AttrEvt>,
  /// 不需要 watch 监听的表达式属性，例如 `<div a={45 + "hello"} b={_someVar.o} c="hello" d={true} disabled ></div>`
  pub const_props: Vec<(IdentName, Box<Expr>)>,
  pub watch_props: Vec<(IdentName, ExprParseResult)>,
}

impl TemplateParser {
  pub fn parse_attrs(&mut self, n: &JSXElement, is_component: bool) -> AttrStore {
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
        if JINGE_CHILDREN.eq(&an.sym) || JINGE_SLOTS.eq(&an.sym) {
          emit_error(an.span(), "警告：不能使用 children 和 slots 属性名，如果是定义  Slot，请使用 jsx 子元素的方式定义！");
        } else if JINGE_LOOP_KEY.eq(&an.sym) {
          // 当前版本 key 属性暂时仅用于在语法层面兼容 react/vue，实际没有作用，直接忽略。
          // 列表循环使用的 <For> 组件，等价的属性为 `keyFn` 属性。
          // 此外，map 函数会被编译器自动转换为 <For> 组件，转换时会找到元素上的 key 属性也同时转换为 keyFn 属性。参看 ./map.rs 和 ./map_key.rs
        } else if JINGE_REF.eq(&an.sym) {
          if attrs.ref_prop.is_some() {
            emit_error(attr.span(), "不能重复指定 ref");
            return;
          }
      
          let Some(JSXAttrValue::JSXExprContainer(val)) = &attr.value else {
            emit_error(attr.value.span(), "ref 属性值不合法");
            return;
          };
         
          let JSXExpr::Expr(val) = &val.expr else {
            emit_error(val.expr.span(), "ref 属性值不合法");
            return;
          };
          attrs.ref_prop.replace(val.clone());
        } else if !is_component // html 元素的事件处理要单独处理，把 onClick 等转换成 click 事件。Component 元素的事件处理也是标准的属性传递。
          && an.sym.starts_with("on")
          && matches!(an.sym.chars().nth(2), Some(c) if c >= 'A' && c <= 'Z')
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
          let mut event_name = &an.sym[2..];
          let mut capture = false;
          if event_name.ends_with("Capture") {
            event_name = &event_name[..event_name.len() - 7];
            capture = true;
          }
          attrs.evt_props.push(AttrEvt {
            event_name: event_name.to_lowercase().into(),
            event_handler: val.clone(),
            capture,
          })
        } else {
          let attr_name = if !is_component && JINGE_CLASSNAME.eq(&an.sym) {
            IdentName::from(JINGE_CLASS.clone())
          } else {
            an.clone()
          };
          if let Some(val) = &attr.value {
            match val {
              JSXAttrValue::Lit(val) => {
                attrs
                  .const_props
                  .push((attr_name, Box::new(Expr::Lit(val.clone()))));
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
                      .push((attr_name, Box::new(Expr::Lit(val.clone()))));
                  }
                  Expr::Fn(_) | Expr::Arrow(_) => {
                    if !is_component {
                      emit_error(
                        attr.name.span(),
                        "不支持函数作为属性值。如果是想传递事件，请使用 on 打头的属性名，例如 onClick",
                      )
                    } else {
                      let mut set: HashSet<Atom> = HashSet::new();
                      match expr.as_ref() {
                        Expr::Fn(e) => {
                           e.function.params.iter().for_each(|p| {
                             if let Pat::Ident(id) = &p.pat {
                               set.insert(id.sym.clone());
                             }
                           })
                        },
                        Expr::Arrow(e) => {
                          e.params.iter().for_each(|p| {
                            if let Pat::Ident(id) = p {
                              set.insert(id.sym.clone());
                            }
                          })
                        },
                        _ => ()
                      }
                      // println!("{:?}", set);
                      let r = ExprVisitor::new_with_exclude_roots(if set.is_empty() {
                        None
                      } else {
                        Some(Rc::new(set))
                      }).parse(expr.as_ref());
                      match r {
                        ExprParseResult::None => {
                          attrs.const_props.push((attr_name, expr.clone()));
                        }
                        _ => {
                          attrs.watch_props.push((attr_name, r))
                        }
                      }
                    }
                  }
                  _ => {
                    let r = ExprVisitor::new().parse(expr.as_ref());
                    match r {
                      ExprParseResult::None => {
                        attrs.const_props.push((attr_name, expr.clone()));
                      }
                      _ => {
                        attrs.watch_props.push((attr_name, r))
                      }
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
              .push((attr_name, Box::new(Expr::Lit(Lit::Bool(Bool::from(true))))));
          }
        }
      }
    });

    attrs
  }
}

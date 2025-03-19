use crate::ast::*;
use crate::common::{
  IntlType, JINGE_ATTR_IDENT, JINGE_KEY, JINGE_ON, JINGE_SLOT, JINGE_T, emit_error,
};
use crate::helper::parse_intl_call;
use crate::parser::TemplateParser;
use swc_common::DUMMY_SP;
use swc_core::ecma::ast::*;
use swc_core::{atoms::Atom, common::Spanned};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};

use super::expr::{ExprParseResult, ExprVisitor};
use super::slot::{SlotName, get_slot_name_from_member_expr, slot_name_to_callee_expr};
use super::{
  JINGE_CHILDREN, JINGE_CLASS, JINGE_CLASSNAME, JINGE_FOR, JINGE_HTML_FOR, JINGE_REF, Parent, Slot,
};

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
  /// ... 解构写法透传的属性，例如 `<A {...state} />` 本质上就是把 state 作为 A 组件的 props 参数直接传递。
  /// 这种写法的情况下，不能再有其它 const 或 watch 属性，并且只能出现一次。
  pub spread_prop: Option<Ident>,
  /// Slot 属性
  pub slot_props: Vec<Slot>,
  pub meet_slot: bool,
}

impl TemplateParser {
  fn parse_ref_attr(&self, attrs: &mut AttrStore, attr: &JSXAttr) {
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
  }
  fn parse_key_attr(&self, attrs: &mut AttrStore, attr: &JSXAttr) {
    const KEY_ERROR: &str = "key 属性值不合法";
    let Some(val) = &attr.value else {
      emit_error(attr.value.span(), KEY_ERROR);
      return;
    };

    let expr = match val {
      JSXAttrValue::Lit(Lit::Str(v)) => Box::new(Expr::Lit(Lit::Str(v.clone()))),
      JSXAttrValue::JSXExprContainer(expr) => match &expr.expr {
        JSXExpr::Expr(expr) => expr.clone(),
        JSXExpr::JSXEmptyExpr(_) => {
          emit_error(val.span(), KEY_ERROR);
          return;
        }
      },
      _ => {
        emit_error(val.span(), KEY_ERROR);
        return;
      }
    };
    attrs.const_props.push((JINGE_KEY.clone().into(), expr));
  }
  fn parse_prop_attr(
    &self,
    attrs: &mut AttrStore,
    an: &IdentName,
    av: &Option<JSXAttrValue>,
    is_component: bool,
  ) {
    let attr_name = if !is_component {
      if JINGE_CLASSNAME.eq(&an.sym) {
        IdentName::from(JINGE_CLASS.clone())
      } else if JINGE_HTML_FOR.eq(&an.sym) {
        IdentName::from(JINGE_FOR.clone())
      } else {
        an.clone()
      }
    } else {
      an.clone()
    };

    let Some(val) = av else {
      // 没有属性值，当 bool 类型处理。
      attrs
        .const_props
        .push((attr_name, Box::new(Expr::Lit(Lit::Bool(Bool::from(true))))));
      return;
    };

    match val {
      JSXAttrValue::Lit(val) => {
        attrs
          .const_props
          .push((attr_name, Box::new(Expr::Lit(val.clone()))));
      }
      JSXAttrValue::JSXExprContainer(val) => match &val.expr {
        JSXExpr::JSXEmptyExpr(_) => (),
        JSXExpr::Expr(expr) => match expr.as_ref() {
          Expr::JSXEmpty(_) => (),
          Expr::JSXMember(_) | Expr::JSXNamespacedName(_) => {
            emit_error(an.span(), "不支持该类型的 JSX 定义。");
          }
          Expr::JSXElement(_) | Expr::JSXFragment(_) => {
            emit_error(
              an.span(),
              &format!(
                "属性值不支持 JSX 元素，如果是定义插槽请使用 slot:{} 定义。",
                an.sym
              ),
            );
          }
          Expr::Lit(val) => {
            attrs
              .const_props
              .push((attr_name, Box::new(Expr::Lit(val.clone()))));
          }
          Expr::Fn(_) | Expr::Arrow(_) => {
            if !attr_name.sym.starts_with('_') {
              emit_error(
                an.span(),
                "函数作为属性值时，属性名必须是 _ 打头的单向属性名。即，当该函数体内部依赖到的数据变更时，不会触发该属性的变更。",
              );
            } else {
              // 属性值如果是函数，则直接赋值。注意，当函数体内部依赖到的数据的变更时不会触发该属性的变更。
              attrs.const_props.push((attr_name, expr.clone()));
            }
          }
          _ => {
            let r = ExprVisitor::new().parse(expr.as_ref());
            match r {
              ExprParseResult::None => {
                attrs.const_props.push((attr_name, expr.clone()));
              }
              _ => attrs.watch_props.push((attr_name, r)),
            }
          }
        },
      },
      _ => emit_error(
        val.span(),
        &format!(
          "属性值不支持 JSX 元素，如果是定义插槽请使用 slot:{} 定义。",
          an.sym
        ),
      ),
    }
  }
  fn meet_slot(&mut self, attrs: &mut AttrStore, an: &IdentName) {
    if !attrs.meet_slot {
      attrs.meet_slot = true;
      self.push_context(Parent::Component);
    }
    self.context.slots.push(Slot::new(an.sym.clone()));
  }
  fn parse_slot_attr(&mut self, attrs: &mut AttrStore, an: &IdentName, av: &JSXAttrValue) {
    match av {
      JSXAttrValue::Lit(val) => {
        self.meet_slot(attrs, an);
        self.visit_lit(val);
      }
      JSXAttrValue::JSXExprContainer(val) => match &val.expr {
        JSXExpr::JSXEmptyExpr(_) => (),
        JSXExpr::Expr(expr) => match expr.as_ref() {
          Expr::JSXEmpty(_) | Expr::JSXMember(_) | Expr::JSXNamespacedName(_) => {
            // ignore
          }
          Expr::JSXElement(_) | Expr::JSXFragment(_) => {
            self.meet_slot(attrs, an);
            self.visit_expr(expr);
          }
          Expr::Lit(val) => {
            self.meet_slot(attrs, an);
            self.visit_lit(val);
          }
          Expr::Fn(expr) => {
            self.meet_slot(attrs, an);
            self.parse_func_function(expr);
          }
          Expr::Arrow(expr) => {
            self.meet_slot(attrs, an);
            self.parse_func_arrow(expr);
          }
          Expr::Member(mem_expr) => {
            // 如果 slot: 类型的属性，值是 member 表达式，则有可能是二次传递插槽。
            // 比如 <B slot:x={props.children} /> 或 <B slot:x={a.b['slot:k']} />
            let slot_name = get_slot_name_from_member_expr(mem_expr, &self.props_arg);
            match slot_name {
              SlotName::None => {
                self.meet_slot(attrs, an);
                self.visit_expr(expr);
              }
              _ => {
                self.meet_slot(attrs, an);
                self
                  .context
                  .slots
                  .last_mut()
                  .unwrap()
                  .pass_by
                  .replace(slot_name_to_callee_expr(slot_name));
              }
            }
          }
          _ => {
            self.meet_slot(attrs, an);
            self.visit_expr(expr);
          }
        },
      },
      _ => emit_error(an.span(), "slot属性不支持该类型的属性值。"),
    }
  }
  fn parse_event_attr(
    &self,
    attrs: &mut AttrStore,
    an: &IdentName,
    av: &Option<JSXAttrValue>,
    is_component: bool,
  ) {
    let Some(JSXAttrValue::JSXExprContainer(val)) = av else {
      emit_error(an.span(), "事件属性的属性值必须是函数或表达式");
      return;
    };
    let JSXExpr::Expr(val) = &val.expr else {
      emit_error(an.span(), "事件属性的属性值必须是函数或表达式");
      return;
    };
    let mut event_handler = match val.as_ref() {
      Expr::Arrow(_) | Expr::Fn(_) => val.clone(),
      Expr::JSXElement(_)
      | Expr::JSXFragment(_)
      | Expr::Await(_)
      | Expr::JSXMember(_)
      | Expr::JSXEmpty(_)
      | Expr::Lit(_) => {
        emit_error(an.span(), "事件属性的属性值必须是函数或表达式");
        return;
      }
      _ => {
        let x = Pat::Rest(RestPat {
          span: DUMMY_SP,
          dot3_token: DUMMY_SP,
          arg: Box::new(Pat::Ident(BindingIdent::from(JINGE_ATTR_IDENT.clone()))),
          type_ann: None,
        });
        let x2 = ExprOrSpread {
          spread: Some(DUMMY_SP),
          expr: ast_create_expr_ident(JINGE_ATTR_IDENT.clone()),
        };
        ast_create_expr_arrow_fn(
          vec![x],
          Box::new(BlockStmtOrExpr::Expr(ast_create_expr_call(
            val.clone(),
            vec![x2],
          ))),
        )
      }
    };

    if let IntlType::Enabled(drop_default_text) = self.intl_type {
      let mut intl_parser = EventHandlerIntlParser { drop_default_text };
      intl_parser.visit_mut_expr(&mut event_handler);
    }

    if is_component {
      attrs
        .const_props
        .push((IdentName::from(format!("on:{}", an.sym)), event_handler));
    } else {
      let mut event_name = an.sym.as_str();
      let mut capture = false;
      if event_name.ends_with("Capture") {
        event_name = &event_name[..event_name.len() - 7];
        capture = true;
      }
      let event_name = Atom::from(event_name);
      attrs.evt_props.push(AttrEvt {
        event_name,
        event_handler,
        capture,
      });
    };
  }
  pub fn parse_attrs(&mut self, n: &JSXElement, is_component: bool) -> AttrStore {
    let mut attrs = AttrStore {
      ref_prop: None,
      evt_props: vec![],
      const_props: vec![],
      watch_props: vec![],
      spread_prop: None,
      slot_props: vec![],
      meet_slot: false,
    };

    n.opening.attrs.iter().for_each(|attr| match attr {
      JSXAttrOrSpread::SpreadElement(s) => {
        let Expr::Ident(id) = s.expr.as_ref() else {
          emit_error(s.span(), "解构写法...后必须是 Ident");
          return;
        };
        if attrs.spread_prop.is_some() {
          emit_error(s.span(), "解构写法透传属性只能出现一次");
        } else {
          attrs.spread_prop.replace(id.clone());
        }
      }
      JSXAttrOrSpread::JSXAttr(attr) => match &attr.name {
        JSXAttrName::Ident(an) => {
          if JINGE_CHILDREN.eq(&an.sym) {
            emit_error(
              an.span(),
              "警告：不能使用 children 属性名，请使用 jsx 子元素的方式定义默认 Slot 子元素！",
            );
          } else if JINGE_REF.eq(&an.sym) {
            self.parse_ref_attr(&mut attrs, attr);
          } else if JINGE_KEY.eq(&an.sym) {
            self.parse_key_attr(&mut attrs, attr);
          } else {
            self.parse_prop_attr(&mut attrs, an, &attr.value, is_component);
          }
        }
        JSXAttrName::JSXNamespacedName(an) => {
          if JINGE_ON.eq(&an.ns.sym) {
            self.parse_event_attr(&mut attrs, &an.name, &attr.value, is_component);
          } else if JINGE_SLOT.eq(&an.ns.sym) {
            if !is_component {
              emit_error(an.span(), "html 元素不支持 slot: 属性");
            } else if let Some(av) = &attr.value {
              self.parse_slot_attr(&mut attrs, &an.name, av);
            } else {
              emit_error(an.span(), "属性值不能为空");
            }
          } else {
            emit_error(
              an.span(),
              "不支持该命名空间属性名。目前命名空间只支持 on: 或 slot: 。",
            );
          }
        }
      },
    });

    if attrs.meet_slot {
      let mut children_context = self.pop_context();
      attrs.slot_props.append(&mut children_context.slots);
    }

    if attrs.spread_prop.is_some()
      && (!attrs.const_props.is_empty() || !attrs.watch_props.is_empty())
    {
      let id = attrs.spread_prop.take();
      emit_error(id.span(), "解构写法透传属性只能出现一次");
    }
    attrs
  }
}

struct EventHandlerIntlParser {
  drop_default_text: bool,
}

impl VisitMut for EventHandlerIntlParser {
  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    let Callee::Expr(callee) = &node.callee else {
      node.visit_mut_children_with(self);
      return;
    };
    if !matches!(callee.as_ref(), Expr::Ident(name) if JINGE_T.eq(&name.sym)) {
      node.visit_mut_children_with(self);
      return;
    }
    parse_intl_call(node, self.drop_default_text);
  }
}

use swc_common::Spanned;
use swc_core::{atoms::Atom, ecma::ast::*};

use crate::common::emit_warn;

use super::{emit_error, JINGE_KEY};

const BAD_KEY_WARNING: &'static str =
  "已忽略 Key。Key 的表达式必须是 map 函数的参数或参数的属性表达式";

#[derive(Debug)]
pub enum MapKey {
  Data,
  Index,
  Prop(String),
  None,
}
impl MapKey {
  pub fn is_none(&self) -> bool {
    match self {
      MapKey::None => true,
      _ => false,
    }
  }
}
/// 将 map 函数体返回的第一个有 key 属性的 jsx 元素的 key 属性的表达式，转换成 <For> 组件的 keyFn 属性。
/// 要求 key 属性必须是 Ident 或 MemberExpr，且必须是 map 函数的第一个 data 参数或第二个 index 参数。
///
pub struct MapKeyFindVisitor {
  pub arg_data: Option<Atom>,
  pub arg_index: Option<Atom>,
}
impl MapKeyFindVisitor {
  // #[inline]
  // fn get_key_fn_params(&self) -> Vec<Pat> {
  //   let mut params = vec![];
  //   if let Some(v) = &self.arg_data {
  //     params.push(Pat::Ident(BindingIdent::from(v.clone())));
  //   }
  //   if let Some(v) = &self.arg_index {
  //     params.push(Pat::Ident(BindingIdent::from(v.clone())));
  //   }
  //   params
  // }
  fn get_key_from_jsx_element(&self, expr: &JSXElement) -> MapKey {
    for attr in expr.opening.attrs.iter() {
      let JSXAttrOrSpread::JSXAttr(attr) = attr else {
        continue;
      };
      if !matches!(&attr.name, JSXAttrName::Ident(id) if JINGE_KEY.eq(&id.sym)) {
        continue;
      }
      if self.arg_data.is_none() && self.arg_index.is_none() {
        emit_warn(
          attr.span(),
          "map 函数没有指定参数，因此 key 属性无法转换为 <For> 组件的 key 参数。",
        );
        return MapKey::None;
      }
      let Some(JSXAttrValue::JSXExprContainer(expr)) = &attr.value else {
        emit_error(attr.span(), BAD_KEY_WARNING);
        return MapKey::None;
      };
      let JSXExpr::Expr(expr) = &expr.expr else {
        emit_error(expr.span(), BAD_KEY_WARNING);
        return MapKey::None;
      };
      return match expr.as_ref() {
        Expr::Member(e) => {
          let mut root = e;
          let mut path = String::new();
          loop {
            match &root.prop {
              MemberProp::Ident(id) => {
                let mut x = id.sym.to_string();
                if !path.is_empty() {
                  x.push('.');
                  x.push_str(&path);
                }
                path = x;
              }
              MemberProp::Computed(e) => match e.expr.as_ref() {
                Expr::Lit(Lit::Str(v)) => {
                  let mut x = "['".to_string();
                  x.push_str(&v.value.replace('\'', "\\'"));
                  x.push_str("']");
                  if !path.is_empty() {
                    if !path.starts_with('[') {
                      x.push('.');
                    }
                    x.push_str(&path);
                  }
                  path = x;
                }
                _ => {
                  emit_warn(e.span(), BAD_KEY_WARNING);
                  return MapKey::None;
                }
              },
              MemberProp::PrivateName(k) => {
                emit_warn(k.span(), BAD_KEY_WARNING);
                return MapKey::None;
              }
            }
            match root.obj.as_ref() {
              Expr::Member(e) => root = e,
              _ => break,
            }
          }
          if let Expr::Ident(id) = root.obj.as_ref() {
            if self
              .arg_data
              .as_ref()
              .map(|v| id.sym.eq(v))
              .unwrap_or(false)
            {
              MapKey::Prop(path)
            } else {
              emit_warn(expr.span(), BAD_KEY_WARNING);
              MapKey::None
            }
          } else {
            emit_warn(e.span(), BAD_KEY_WARNING);
            MapKey::None
          }
        }
        Expr::Ident(id) => {
          if self
            .arg_data
            .as_ref()
            .map(|v| id.sym.eq(v))
            .unwrap_or(false)
          {
            MapKey::Data
          } else if self
            .arg_index
            .as_ref()
            .map(|v| id.sym.eq(v))
            .unwrap_or(false)
          {
            MapKey::Index
          } else {
            emit_warn(expr.span(), BAD_KEY_WARNING);
            MapKey::None
          }
        }
        _ => {
          emit_warn(expr.span(), BAD_KEY_WARNING);
          MapKey::None
        }
      };
    }
    MapKey::None
  }

  fn get_key_from_jsx_fragment(&self, expr: &JSXFragment) -> MapKey {
    for child in expr.children.iter() {
      let rtn = match child {
        JSXElementChild::JSXExprContainer(e) => match &e.expr {
          JSXExpr::Expr(e) => self.get_key_inner(e.as_ref()),
          JSXExpr::JSXEmptyExpr(_) => MapKey::None,
        },
        JSXElementChild::JSXElement(e) => self.get_key_from_jsx_element(e.as_ref()),
        JSXElementChild::JSXFragment(fe) => self.get_key_from_jsx_fragment(fe),
        _ => MapKey::None,
      };
      if !rtn.is_none() {
        return rtn;
      }
    }
    MapKey::None
  }

  fn get_key_inner(&self, expr: &Expr) -> MapKey {
    match expr {
      Expr::Paren(e) => self.get_key_inner(e.expr.as_ref()),
      Expr::JSXFragment(fe) => self.get_key_from_jsx_fragment(fe),
      Expr::JSXElement(e) => self.get_key_from_jsx_element(e.as_ref()),
      _ => MapKey::None,
    }
  }

  pub fn get_key(&self, expr: &ArrowExpr) -> MapKey {
    let BlockStmtOrExpr::Expr(expr) = expr.body.as_ref() else {
      // self.parse_component_element 里会约束 Slot 函数只能是箭头函数且箭头函数直接返回 Expr 表达式。
      // 所以如果是 expr.body 是 BlockStmt 则不再需要尝试获取 key 属性。
      return MapKey::None;
    };
    self.get_key_inner(expr)
  }
}

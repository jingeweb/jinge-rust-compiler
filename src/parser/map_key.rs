use swc_common::Spanned;
use swc_core::{atoms::Atom, ecma::ast::*};

use crate::common::emit_warn;

use super::{JINGE_KEY, emit_error};

const BAD_KEY_WARNING: &'static str = "key 不是受支持的表达式，已忽略。";

#[derive(Debug)]
pub enum MapKey {
  Data,
  Index,
  Prop(String),
  Expr(Box<Expr>),
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
  fn get_key_from_jsx_element(&self, attrs: &Vec<JSXAttrOrSpread>) -> (usize, MapKey) {
    for (index, attr) in attrs.iter().enumerate() {
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
        return (0, MapKey::None);
      }
      if let Some(JSXAttrValue::Lit(expr)) = &attr.value {
        match expr {
          Lit::Str(s) => {
            return (index, MapKey::Prop(s.value.to_string()));
          }
          _ => {
            emit_error(attr.span(), BAD_KEY_WARNING);
            return (0, MapKey::None);
          }
        }
      }
      let Some(JSXAttrValue::JSXExprContainer(expr)) = &attr.value else {
        emit_error(attr.span(), BAD_KEY_WARNING);
        return (0, MapKey::None);
      };
      let JSXExpr::Expr(expr) = &expr.expr else {
        emit_error(expr.span(), BAD_KEY_WARNING);
        return (0, MapKey::None);
      };
      return match expr.as_ref() {
        Expr::Member(me) => {
          let mut root = me;
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
                  return (0, MapKey::None);
                }
              },
              MemberProp::PrivateName(k) => {
                emit_warn(k.span(), BAD_KEY_WARNING);
                return (0, MapKey::None);
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
              (index, MapKey::Prop(path))
            } else {
              // emit_warn(expr.span(), BAD_KEY_WARNING);
              // MapKey::None
              (index, MapKey::Expr(Box::new(Expr::Member(me.clone()))))
            }
          } else {
            emit_warn(me.span(), BAD_KEY_WARNING);
            (0, MapKey::None)
          }
        }
        Expr::Ident(id) => {
          if self
            .arg_data
            .as_ref()
            .map(|v| id.sym.eq(v))
            .unwrap_or(false)
          {
            (index, MapKey::Data)
          } else if self
            .arg_index
            .as_ref()
            .map(|v| id.sym.eq(v))
            .unwrap_or(false)
          {
            (index, MapKey::Index)
          } else {
            (index, MapKey::Expr(Box::new(Expr::Ident(id.clone()))))
          }
        }
        Expr::Lit(Lit::Str(k)) => (index, MapKey::Prop(k.value.to_string())),
        _ => {
          emit_warn(expr.span(), BAD_KEY_WARNING);
          (0, MapKey::None)
        }
      };
    }
    (0, MapKey::None)
  }

  fn get_key_from_jsx_fragment(&self, expr: &mut JSXFragment) -> MapKey {
    for child in expr.children.iter_mut() {
      let rtn = match child {
        JSXElementChild::JSXExprContainer(e) => match &mut e.expr {
          JSXExpr::Expr(e) => self.get_key_inner(e.as_mut()),
          JSXExpr::JSXEmptyExpr(_) => MapKey::None,
        },
        JSXElementChild::JSXElement(e) => {
          let attrs = &mut e.opening.attrs;
          let (index, key) = self.get_key_from_jsx_element(attrs);
          if !key.is_none() {
            attrs.remove(index);
          }
          key
        }
        JSXElementChild::JSXFragment(fe) => self.get_key_from_jsx_fragment(fe),
        _ => MapKey::None,
      };
      if !rtn.is_none() {
        return rtn;
      }
    }
    MapKey::None
  }

  fn get_key_inner(&self, expr: &mut Expr) -> MapKey {
    match expr {
      Expr::Paren(e) => self.get_key_inner(e.expr.as_mut()),
      Expr::JSXFragment(fe) => self.get_key_from_jsx_fragment(fe),
      Expr::JSXElement(e) => {
        let attrs = &mut e.opening.attrs;
        let (index, key) = self.get_key_from_jsx_element(attrs);
        if !key.is_none() {
          attrs.remove(index);
        }
        key
      }
      _ => MapKey::None,
    }
  }

  pub fn get_key(&self, expr: &mut ArrowExpr) -> MapKey {
    let BlockStmtOrExpr::Expr(expr) = expr.body.as_mut() else {
      // self.parse_component_element 里会约束 Slot 函数只能是箭头函数且箭头函数直接返回 Expr 表达式。
      // 所以如果是 expr.body 是 BlockStmt 则不再需要尝试获取 key 属性。
      return MapKey::None;
    };
    self.get_key_inner(expr)
  }
}

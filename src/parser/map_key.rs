use std::borrow::Cow;

use hashbrown::HashSet;
use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_ecma_visit::{Visit, VisitWith};

use crate::common::emit_warn;

use super::{JINGE_KEY, emit_error};

const BAD_KEY_WARNING: &str = "key 不是受支持的表达式，已忽略。";

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
    matches!(self, MapKey::None)
  }
}
/// 将 map 函数体返回的第一个有 key 属性的 jsx 元素的 key 属性的表达式，转换成 <For> 组件的 keyFn 属性。
/// 要求 key 属性必须是 Ident 或 MemberExpr，且必须是 map 函数的第一个 data 参数或第二个 index 参数。
///
pub struct MapKeyFindVisitor {
  pub arg_data: Option<Id>,
  pub arg_index: Option<Id>,
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
  fn get_key_from_jsx_element(&self, attrs: &[JSXAttrOrSpread]) -> (usize, MapKey) {
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
      if let Some(JSXAttrValue::Str(expr)) = &attr.value {
        return (
          index,
          MapKey::Prop(match expr.value.to_string_lossy() {
            Cow::Borrowed(v) => v.to_string(),
            Cow::Owned(v) => v,
          }),
        );
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
                  if !path.starts_with('[') {
                    x.push('.');
                  }
                  x.push_str(&path);
                }
                path = x;
              }
              MemberProp::Computed(e) => match e.expr.as_ref() {
                Expr::Lit(Lit::Str(v)) => {
                  let Some(p) = v.value.as_str() else {
                    emit_warn(e.span(), BAD_KEY_WARNING);
                    return (0, MapKey::None);
                  };
                  let mut x = "['".to_string();
                  x.push_str(
                    &p.replace('\\', "\\\\")
                      .replace('\'', "\\'")
                      .replace('\n', "\\n")
                      .replace('\r', "\\r")
                      .replace('\u{2028}', "\\u2028")
                      .replace('\u{2029}', "\\u2029"),
                  );
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
              .map(|v| id.to_id().eq(v))
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
            .map(|v| id.to_id().eq(v))
            .unwrap_or(false)
          {
            (index, MapKey::Data)
          } else if self
            .arg_index
            .as_ref()
            .map(|v| id.to_id().eq(v))
            .unwrap_or(false)
          {
            (index, MapKey::Index)
          } else {
            (index, MapKey::Expr(Box::new(Expr::Ident(id.clone()))))
          }
        }
        Expr::Lit(Lit::Str(k)) => {
          let Some(p) = k.value.as_str() else {
            emit_warn(expr.span(), BAD_KEY_WARNING);
            return (0, MapKey::None);
          };
          (index, MapKey::Prop(p.to_string()))
        }
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
    let mut locals = KeyLocals::default();
    expr.body.visit_with(&mut locals);
    let key = match expr.body.as_mut() {
      ArrowFunctionBody::Expr(expr) => self.get_key_inner(expr),
      ArrowFunctionBody::FunctionBody(body) => {
        match body
          .stmts
          .iter_mut()
          .rev()
          .find(|s| !matches!(s, Stmt::Empty(_)))
        {
          Some(Stmt::Return(r)) => r
            .arg
            .as_mut()
            .map(|e| self.get_key_inner(e))
            .unwrap_or(MapKey::None),
          _ => MapKey::None,
        }
      }
    };
    if let MapKey::Expr(value) = &key {
      let mut root = value.as_ref();
      while let Expr::Member(member) = root {
        root = &member.obj;
      }
      if let Expr::Ident(id) = root
        && locals.0.contains(&id.to_id())
      {
        emit_error(
          value.span(),
          &format!(
            "key 引用了 map 回调内的局部变量，暂不支持向 For 提取；请直接完整书写 key 表达式，例如 key={{{}.id}}，不要先提取为局部变量",
            self
              .arg_data
              .as_ref()
              .map(|id| id.0.as_ref())
              .unwrap_or("item")
          ),
        );
        return MapKey::None;
      }
    }
    key
  }
}

#[derive(Default)]
struct KeyLocals(HashSet<Id>);
impl Visit for KeyLocals {
  fn visit_binding_ident(&mut self, id: &BindingIdent) {
    self.0.insert(id.id.to_id());
  }
  fn visit_fn_decl(&mut self, node: &FnDecl) {
    self.0.insert(node.ident.to_id());
    node.visit_children_with(self);
  }
  fn visit_class_decl(&mut self, node: &ClassDecl) {
    self.0.insert(node.ident.to_id());
    node.visit_children_with(self);
  }
}

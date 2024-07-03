use std::ops::{Deref, DerefMut};

use crate::visitor::TransformVisitor;
use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha512};
use swc_core::{
  common::{util::take::Take, Spanned, DUMMY_SP},
  ecma::ast::{Callee, Expr, Ident, KeyValueProp, Lit, Prop, PropName, PropOrSpread, Str},
};

/// 计算文本的 hash。
/// 需要和 packages/tools/intl/extract/helper.rs 中使用算法一致，当前统一为 sha512().toBase64().slice(0,6)。
/// 如果修改两处都要变更。
fn calc_key(message: &str, filename: Option<&str>) -> String {
  let mut h = Sha512::new();
  h.update(message);
  if let Some(filename) = filename {
    h.update(filename)
  }
  let h = h.finalize();
  // println!("hash len: {:?}", h.len());
  let mut enc_buf = [0u8; 128];

  let x = Base64::encode(&h, &mut enc_buf).unwrap();
  x[0..6].to_string()
}

pub fn visit_mut_call_expr(visitor: &mut TransformVisitor, n: &mut swc_core::ecma::ast::CallExpr) {
  let mut is_use_intl = false;
  if let Callee::Expr(expr) = &n.callee {
    let expr = expr.deref();
    if let Expr::Ident(fn_name) = expr {
      if fn_name.sym.eq("useIntl") {
        is_use_intl = true;
      }
    }
  }
  if !is_use_intl || n.args.is_empty() {
    return;
  }
  let arg = &mut n.args[0];
  let arg = arg.expr.deref_mut();

  if let Expr::Object(expr) = arg {
    let mut default_message = None;
    let mut key_name = None;
    let mut isolated = false;

    expr.props.iter_mut().for_each(|prop| {
      let mut is_default_message = false;
      let mut is_key = false;
      let mut is_isolated = false;
      if let PropOrSpread::Prop(prop) = prop {
        let prop = prop.deref().deref();
        if let Prop::KeyValue(prop) = prop {
          if let PropName::Ident(key) = &prop.key {
            if key.sym.eq("defaultMessage") {
              is_default_message = true;
              let value = prop.value.deref();
              if let Expr::Lit(Lit::Str(value)) = value {
                default_message = Some(value.value.to_string())
              }
            } else if key.sym.eq("key") {
              is_key = true;
              let value = prop.value.deref();
              if let Expr::Lit(Lit::Str(value)) = value {
                key_name = Some(value.value.to_string())
              }
            } else if key.sym.eq("isolated") {
              is_isolated = true;
              let value = prop.value.deref();
              if let Expr::Lit(Lit::Bool(value)) = value {
                isolated = value.value
              }
            }
          }
        }
      }

      if is_default_message && matches!(visitor.config.delete_default_message, Some(true)) {
        prop.take();
      }
      if is_isolated || is_key {
        prop.take();
      }
    });

    // println!("{:?}", expr.props.len());

    expr.props.retain(|prop| match prop {
      PropOrSpread::Spread(s) => !s.dot3_token.eq(&DUMMY_SP),
      _ => true,
    });

    // println!("AFTER {:?}", expr.props.len());

    if key_name.is_none() && default_message.is_some() {
      let filename = if isolated {
        Some(visitor.filename.as_str())
      } else {
        None
      };
      key_name = Some(calc_key(default_message.as_ref().unwrap(), filename));
    }

    let mut span = expr.span();
    span.hi.0 -= 1;
    span.lo.0 = span.hi.0 - 1;

    expr
      .props
      .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("key".into(), span.span())),
        value: Box::new(Expr::Lit(Lit::Str(Str {
          value: key_name.unwrap().into(),
          span: span.span(),
          raw: None,
        }))),
      }))))
  }
}

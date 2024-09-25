use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha512};
use swc_common::Spanned;
use swc_core::ecma::ast::{Expr, ExprOrSpread, Lit, Prop, PropName, PropOrSpread};

use super::{emit_error, tpl_render_intl_text, TemplateParser, JINGE_KEY, JINGE_T};

/// 计算文本的 hash。
/// 需要和 /ts/intl/extract/helper.rs 中使用算法一致，当前统一为 sha512().toBase64().slice(0,6)。
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

impl TemplateParser {
  /// 将国际化多语言的 t 函数转换为相应的组件或渲染。这里采用了极简单的粗糙方法，仅通过函数名为 t 来判定。
  /// 因此有很大的问题，比如不支持 `import {t as someFn} from 'jinge'` 的别名 import 写法；
  /// 比如如果用户使用了自已定义的也名为 t 函数。
  /// TODO: 未来结合实现情况来支持上述两种 case。
  pub fn parse_intl_t(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    if !matches!(callee, Expr::Ident(name) if JINGE_T.eq(&name.sym)) {
      return false;
    }
    let Some(default_text) = args.get(0) else {
      return false;
    };
    if default_text.spread.is_some() {
      return false;
    }
    let Expr::Lit(default_text) = default_text.expr.as_ref() else {
      return false;
    };
    let Lit::Str(default_text) = default_text else {
      return false;
    };
    let default_text = &default_text.value;
    let mut key = None;
    // let mut isolate = false;
    let params = args.get(1);
    let options = args.get(2);
    if let Some(options) = options {
      if options.spread.is_some() {
        emit_error(options.span(), "t 函数的参数不支持 ... 解构写法");
      } else if let Expr::Object(opts) = options.expr.as_ref() {
        for prop in opts.props.iter() {
          if let PropOrSpread::Prop(prop) = prop {
            if let Prop::KeyValue(kv) = prop.as_ref() {
              if let PropName::Ident(id) = &kv.key {
                if JINGE_KEY.eq(&id.sym) {
                  if let Expr::Lit(Lit::Str(v)) = kv.value.as_ref() {
                    key = Some(v.value.clone());
                  }
                  // 找到 key 就退出循环。
                  break;
                }
              }
            }
          }
        }
      }
    }

    if key.is_none() {
      key = Some(calc_key(default_text.as_str(), None).into());
    }

    self.push_expression(tpl_render_intl_text(
      &key.unwrap(),
      params,
      Some(default_text),
      self.context.is_parent_component(),
      self.context.root_container,
    ));

    true
  }
}

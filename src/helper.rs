use swc_core::ecma::ast::{CallExpr, Callee, Expr, ExprOrSpread};

use crate::{
  ast::*,
  common::{JINGE_T, JINGE_UNDEFINED},
  parser::intl::extract_t,
};

pub fn has_jsx(expr: &Expr) -> bool {
  match expr {
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(e) => {
      return has_jsx(&e.alt) || has_jsx(&e.cons);
    }
    Expr::Bin(e) => return has_jsx(&e.left) || has_jsx(&e.right),
    Expr::Paren(e) => return has_jsx(&e.expr),
    Expr::Call(e) => match &e.callee {
      Callee::Expr(e) => match e.as_ref() {
        Expr::Ident(id) if JINGE_T.eq(&id.sym) => true,
        _ => false,
      },
      _ => false,
    },
    _ => {
      return false;
    }
  }
}

/// 将国际化函数转成从字典中取值，例如: t('Hello') 转成 t('[HASH_KEY]')。
/// 这里的转化不考虑监听语言的变更，即仅用于比如事件处理函数体内部的国际化。
pub fn parse_intl_call(node: &mut CallExpr, drop_default_text: bool) {
  let Some((key, default_text, params)) = extract_t(&node.args) else {
    return;
  };

  let mut args = vec![ExprOrSpread {
    spread: None,
    expr: ast_create_expr_lit_str(key),
  }];
  let mut has_params = false;
  if let Some(params) = params {
    has_params = true;
    args.push(ast_create_arg_expr(Box::new(Expr::Object(params.clone()))));
  }
  // println!("OOOO {} {}", self.drop_default_text, default_text);
  if !drop_default_text {
    if !has_params {
      args.push(ast_create_arg_expr(ast_create_expr_ident(
        JINGE_UNDEFINED.clone().into(),
      )));
    }
    args.push(ast_create_arg_expr(ast_create_expr_lit_str(
      default_text.clone(),
    )));
  }

  node.args = args;
}

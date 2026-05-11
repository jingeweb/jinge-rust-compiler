use swc_core::{
  atoms::Atom,
  ecma::ast::{BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Expr, ExprOrSpread, MemberProp, Stmt},
};

use crate::{
  ast::*,
  common::{JINGE_MAP, JINGE_T, JINGE_UNDEFINED},
  parser::{
    intl::extract_t,
    slot::{
      get_slot_name_from_callee, get_slot_name_from_member_expr, get_slot_name_from_optchain_expr,
    },
  },
};

// pub fn has_jsx(expr: &Expr) -> bool {
//   match expr {
//     Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
//     Expr::Cond(e) => {
//       return has_jsx(&e.alt) || has_jsx(&e.cons);
//     }
//     Expr::Bin(e) => return has_jsx(&e.left) || has_jsx(&e.right),
//     Expr::Paren(e) => return has_jsx(&e.expr),
//     Expr::Call(e) => match &e.callee {
//       Callee::Expr(e) => match e.as_ref() {
//         Expr::Ident(id) if JINGE_T.eq(&id.sym) => true,
//         _ => false,
//       },
//       _ => false,
//     },
//     _ => {
//       return false;
//     }
//   }
// }

fn block_stmts_contains_jsx_return(bs: &BlockStmt, props_arg: &Option<Atom>) -> bool {
  let Some(st) = bs.stmts.last() else {
    return false;
  };
  let Stmt::Return(st) = st else {
    return false;
  };
  let Some(expr) = &st.arg else {
    return false;
  };
  should_render_as_jsx(&expr, props_arg)
}

/// 是否是需要 jsx 渲染的表达式。
/// 比如 `<p>x</p>，<></>, {<p>p</p>}` 这一类的直接有 jsx 元素的，
/// 或者 `props.children` 或 `props['slot:a']?.()` 等插槽渲染，
/// 都满足需要转换为 jsx 渲染。
pub fn should_render_as_jsx(expr: &Expr, props_arg: &Option<Atom>) -> bool {
  match expr {
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(e) => {
      return should_render_as_jsx(&e.alt, props_arg) || should_render_as_jsx(&e.cons, props_arg);
    }
    Expr::Bin(e) => {
      return should_render_as_jsx(&e.left, props_arg) || should_render_as_jsx(&e.right, props_arg);
    }
    Expr::Paren(e) => return should_render_as_jsx(&e.expr, props_arg),
    Expr::Member(mem) => {
      props_arg.is_some() && get_slot_name_from_member_expr(mem, props_arg).is_some()
    }
    Expr::OptChain(opt) => {
      props_arg.is_some() && get_slot_name_from_optchain_expr(opt, props_arg).is_some()
    }
    Expr::Call(call) => match &call.callee {
      Callee::Expr(callee) => match callee.as_ref() {
        Expr::Ident(id) if JINGE_T.eq(&id.sym) => true,
        Expr::Member(me) if matches!(&me.prop, MemberProp::Ident(id) if JINGE_MAP.eq(&id.sym)) => {
          let Some(arg_expr) = call.args.first() else {
            return false;
          };
          if arg_expr.spread.is_some() {
            return false;
          }
          let arg_expr = arg_expr.expr.as_ref();
          match arg_expr {
            Expr::Arrow(e) => match e.body.as_ref() {
              BlockStmtOrExpr::Expr(e) => should_render_as_jsx(e.as_ref(), props_arg),
              BlockStmtOrExpr::BlockStmt(bs) => block_stmts_contains_jsx_return(bs, props_arg),
            },
            Expr::Fn(e) => match e.function.body.as_ref() {
              Some(bs) => block_stmts_contains_jsx_return(bs, props_arg),
              _ => false,
            },
            _ => false,
          }
        }
        _ => props_arg.is_some() && get_slot_name_from_callee(callee.as_ref(), props_arg).is_some(),
      },
      _ => false,
    },
    _ => false,
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

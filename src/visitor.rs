use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;
use swc_ecma_visit::VisitMutWith;

use crate::ast::{ast_create_arg_expr, ast_create_expr_ident, ast_create_expr_lit_str};
use crate::common::{emit_error, IntlType, JINGE_IMPORT_MODULE_ITEM, JINGE_T, JINGE_UNDEFINED};
use crate::parser;
use crate::parser::intl::extract_t;

pub struct TemplateTransformVisitor<'a> {
  changed: bool,
  pub parsed_components: &'a mut Vec<String>,
  pub intl_type: IntlType,
}
impl<'a> TemplateTransformVisitor<'a> {
  pub fn new(parsed_components: &'a mut Vec<String>, intl_type: IntlType) -> Self {
    Self {
      parsed_components,
      intl_type,
      changed: false,
    }
  }
  fn v_func(&mut self, fn_name: Option<&Ident>, expr: &mut Function) {
    if let Some(body) = &mut expr.body {
      self.v_func_body(fn_name, body, expr.params.get(0).map(|p| &p.pat));
    };
  }
  fn v_func_body(&mut self, fn_name: Option<&Ident>, body: &mut BlockStmt, prop_arg: Option<&Pat>) {
    let Some(Stmt::Return(stmt)) = body.stmts.last_mut() else {
      return;
    };
    let Some(expr) = &mut stmt.arg else {
      return;
    };
    if is_jsx(expr.as_ref()) {
      self.v_return(fn_name, expr, prop_arg);
    }
  }
  fn v_arrow(&mut self, fn_name: Option<&Ident>, expr: &mut ArrowExpr) {
    match expr.body.as_mut() {
      BlockStmtOrExpr::Expr(e) => {
        if is_jsx(e.as_ref()) {
          self.v_return(fn_name, e, expr.params.get(0));
        }
      }
      BlockStmtOrExpr::BlockStmt(body) => self.v_func_body(fn_name, body, expr.params.get(0)),
    }
  }

  fn v_return(&mut self, fn_name: Option<&Ident>, expr: &mut Box<Expr>, props_arg: Option<&Pat>) {
    let mut visitor = parser::TemplateParser::new(
      props_arg.and_then(|p| {
        if let Pat::Ident(id) = p {
          Some(id.sym.clone())
        } else {
          emit_error(props_arg.span(), "函数组件的 props 参数不能使用解构写法");
          None
        }
      }),
      self.intl_type,
    );
    if let Some(replaced_expr) = visitor.parse(expr.as_mut()) {
      *expr = replaced_expr;
      self.changed = true;
      if let Some(fn_name) = fn_name {
        self.parsed_components.push(fn_name.sym.to_string());
      }
    }
  }
}
impl VisitMut for TemplateTransformVisitor<'_> {
  fn visit_mut_module(&mut self, n: &mut Module) {
    n.body.iter_mut().for_each(|item| match item {
      ModuleItem::ModuleDecl(decl) => match decl {
        ModuleDecl::ExportDecl(decl) => match &mut decl.decl {
          Decl::Fn(func) => self.v_func(Some(&func.ident), func.function.as_mut()),

          Decl::Var(decl) => decl.as_mut().decls.iter_mut().for_each(|decl| {
            if let Some(x) = &mut decl.init {
              let name = match &decl.name {
                Pat::Ident(id) => Some(&id.id),
                _ => {
                  emit_error(decl.name.span(), "警告：非常规命令的函数组件无法使用 HMR");
                  None
                }
              };
              match x.as_mut() {
                Expr::Fn(func) => self.v_func(name, func.function.as_mut()),
                Expr::Arrow(func) => self.v_arrow(name, func),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        ModuleDecl::ExportDefaultDecl(decl) => match &mut decl.decl {
          DefaultDecl::Fn(func) => self.v_func(
            if let Some(n) = &func.ident {
              Some(n)
            } else {
              emit_error(func.span(), "警告：匿名函数组件无法使用 HMR");
              None
            },
            func.function.as_mut(),
          ),
          _ => (),
        },
        _ => (),
      },
      ModuleItem::Stmt(stmt) => match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Fn(decl) => self.v_func(Some(&decl.ident), decl.function.as_mut()),
          Decl::Var(decl) => decl.decls.iter_mut().for_each(|decl| {
            if let Some(init) = decl.init.as_mut() {
              let name = match &decl.name {
                Pat::Ident(id) => Some(&id.id),
                _ => {
                  emit_error(decl.name.span(), "警告：非常规命令的函数组件无法使用 HMR");
                  None
                }
              };
              match init.as_mut() {
                Expr::Fn(func) => self.v_func(name, func.function.as_mut()),
                Expr::Arrow(func) => self.v_arrow(name, func),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        _ => (),
      },
    });

    if self.changed {
      let mut new_items = Vec::with_capacity(n.body.len() + 1);
      new_items.push(JINGE_IMPORT_MODULE_ITEM.clone());
      new_items.append(&mut n.body);

      n.body = new_items;
    }
  }
}

fn is_jsx(expr: &Expr) -> bool {
  match expr {
    Expr::Paren(expr) => is_jsx(expr.expr.as_ref()),
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(expr) => is_jsx(expr.alt.as_ref()) || is_jsx(expr.cons.as_ref()),
    Expr::Bin(expr) => is_jsx(expr.right.as_ref()),
    _ => false,
  }
}

pub struct IntlTransformVisitor {
  drop_default_text: bool,
}
impl IntlTransformVisitor {
  pub fn new(drop_default_text: bool) -> Self {
    IntlTransformVisitor { drop_default_text }
  }
}
impl VisitMut for IntlTransformVisitor {
  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    let Callee::Expr(callee) = &node.callee else {
      node.visit_mut_children_with(self);
      return;
    };
    if !matches!(callee.as_ref(), Expr::Ident(name) if JINGE_T.eq(&name.sym)) {
      node.visit_mut_children_with(self);
      return;
    }
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
    if !self.drop_default_text {
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
}

use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;

use crate::common::{emit_error, JINGE_IMPORT_MODULE_ITEM};
use crate::parser;

pub struct TransformVisitor {
  changed: bool,
}
impl TransformVisitor {
  pub fn new() -> Self {
    Self { changed: false }
  }
}
impl VisitMut for TransformVisitor {
  fn visit_mut_module(&mut self, n: &mut Module) {
    n.body.iter_mut().for_each(|item| match item {
      ModuleItem::ModuleDecl(decl) => match decl {
        ModuleDecl::ExportDecl(decl) => match &mut decl.decl {
          Decl::Fn(func) => self.v_func(func.function.as_mut()),

          Decl::Var(decl) => decl.as_mut().decls.iter_mut().for_each(|decl| {
            if let Some(x) = &mut decl.init {
              match x.as_mut() {
                Expr::Fn(func) => self.v_func(func.function.as_mut()),
                Expr::Arrow(func) => self.v_arrow(func),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        ModuleDecl::ExportDefaultDecl(decl) => match &mut decl.decl {
          DefaultDecl::Fn(func) => self.v_func(func.function.as_mut()),
          _ => (),
        },
        _ => (),
      },
      ModuleItem::Stmt(stmt) => match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Fn(decl) => self.v_func(decl.function.as_mut()),
          Decl::Var(decl) => decl.decls.iter_mut().for_each(|decl| {
            if let Some(decl) = decl.init.as_mut() {
              match decl.as_mut() {
                Expr::Fn(func) => self.v_func(func.function.as_mut()),
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

impl TransformVisitor {
  fn v_func(&mut self, expr: &mut Function) {
    if let Some(body) = &mut expr.body {
      self.v_func_body(body, expr.params.get(0).map(|p| &p.pat));
    };
  }
  fn v_func_body(&mut self, body: &mut BlockStmt, prop_arg: Option<&Pat>) {
    let Some(Stmt::Return(stmt)) = body.stmts.last_mut() else {
      return;
    };
    let Some(expr) = &mut stmt.arg else {
      return;
    };
    if is_jsx(expr.as_ref()) {
      self.v_return(expr, prop_arg);
    }
  }
  fn v_arrow(&mut self, expr: &mut ArrowExpr) {
    match expr.body.as_mut() {
      BlockStmtOrExpr::Expr(e) => {
        if is_jsx(e.as_ref()) {
          self.v_return(e, expr.params.get(0));
        }
      }
      BlockStmtOrExpr::BlockStmt(body) => self.v_func_body(body, expr.params.get(0)),
    }
  }

  fn v_return(&mut self, expr: &mut Box<Expr>, props_arg: Option<&Pat>) {
    let mut visitor = parser::TemplateParser::new(props_arg.and_then(|p| {
      if let Pat::Ident(id) = p {
        Some(id.sym.clone())
      } else {
        emit_error(
          props_arg.span(),
          "警告：函数组件的 props 参数建议不要使用解构写法",
        );
        None
      }
    }));
    if let Some(replaced_expr) = visitor.parse(expr.as_mut()) {
      *expr = replaced_expr;
      self.changed = true;
    }
  }
}

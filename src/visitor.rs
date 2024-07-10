use std::borrow::Borrow;
use std::ops::Deref;
use std::rc::Rc;

use swc_core::atoms::Atom;
use swc_core::common::{Span, Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::parser;
use swc_core::ecma::visit::{Fold, Visit, VisitAll, VisitMut, VisitWith};
use swc_core::plugin::errors::HANDLER;

use crate::ast::{
  ast_create_console_log, ast_create_expr_call, ast_create_expr_ident, ast_create_expr_lit_str,
};
use crate::common::{emit_error, gen_import_jinge, JINGE_IMPORT_CREATE_ELE, JINGE_IMPORT_TEXT_RENDER_FN};
use crate::config::Config;
use crate::tpl::{self, gen_import_jinge, gen_text_render_func};
use swc_core::ecma::visit::VisitMutWith;
use crate::parser;


pub struct TransformVisitor {
  // pub cwd: String,
  // pub filename: String,
  // pub config: Config,
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
          Decl::Class(cls) => self.v_class_decl(cls),
          Decl::Var(decl) => decl.as_mut().decls.iter_mut().for_each(|decl| {
            if let Some(x) = &mut decl.init {
              match x.as_mut() {
                Expr::Class(cls) => self.v_class_expr(cls),
                _ => (),
              }
            }
          }),
          _ => (),
        },
        ModuleDecl::ExportDefaultDecl(decl) => match &mut decl.decl {
          DefaultDecl::Class(cls) => self.v_class_expr(cls),
          _ => (),
        },
        _ => (),
      },
      ModuleItem::Stmt(stmt) => match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Class(decl) => self.v_class_decl(decl),
          Decl::Var(decl) => decl.decls.iter_mut().for_each(|decl| {
            if let Some(decl) = decl.init.as_mut() {
              match decl.as_mut() {
                Expr::Class(cls) => self.v_class_expr(cls),
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
      new_items.push(gen_import_jinge());
      new_items.append(&mut n.body);

      n.body = new_items;
    }
  }
}

fn is_component(n: &Class) -> bool {
  matches!(&n.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
}
impl TransformVisitor {
  fn v_class_expr(&mut self, n: &mut ClassExpr) {
    if !is_component(&n.class) {
      return;
    }
    self.v_class(n.ident.as_ref(), &mut n.class);
  }
  fn v_class_decl(&mut self, n: &mut ClassDecl) {
    if !is_component(&n.class) {
      return;
    }
    self.v_class(Some(&n.ident), &mut n.class);
  }

  fn v_class(&mut self, ident: Option<&Ident>, class: &mut Class) {
    let render = class.body.iter_mut().find(|it| matches!(it, ClassMember::Method(it) if matches!(&it.key, PropName::Ident(it) if it.sym.as_str() == "render")));
    let Some(render) = render else {
      let span = if let Some(ident) = ident {
        ident.span()
      } else {
        class.span()
      };
      emit_error(span, "组件缺失 render() 函数");
      return;
    };
    let render_fn = match render {
      ClassMember::Method(r) => r.function.as_mut(),
      _ => unreachable!(),
    };
    let Some(return_expr) = render_fn.body.as_mut().and_then(|body| {
      if let Some(Stmt::Return(stmt)) = body.stmts.last_mut() {
        Some(stmt)
      } else {
        None
      }
    }) else {
      // 如果最后一条语句不是 return JSX，则不把 render() 函数当成需要处理的渲染模板。
      return;
    };
    let Some(return_arg) = return_expr.arg.as_ref() else {
      return;
    };
    let mut visitor = parser::TemplateParser::new();
    visitor.visit_expr(&*return_arg);
    if !visitor.context.exprs.is_empty() {
      println!("gen render");
      let elems: Vec<Option<ExprOrSpread>> = visitor
        .context
        .exprs
        .into_iter()
        .map(|e| {
          Some(ExprOrSpread {
            spread: None,
            expr: e,
          })
        })
        .collect();
      return_expr.arg.replace(Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems,
      })));
      self.changed = true;
    }
  }
}

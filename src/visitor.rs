use std::borrow::Borrow;
use std::ops::Deref;
use std::rc::Rc;

use swc_core::atoms::Atom;
use swc_core::common::{Span, Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Fold, Visit, VisitAll, VisitMut, VisitWith};
use swc_core::plugin::errors::HANDLER;

use crate::ast::{
  ast_create_console_log, ast_create_expr_call, ast_create_expr_ident, ast_create_expr_lit_str,
};
use crate::common::{JINGE_IMPORT_CREATE_ELE, JINGE_IMPORT_TEXT_RENDER_FN};
use crate::config::Config;
use crate::tpl::{self, gen_import_jinge, gen_text_render_func};
use swc_core::ecma::visit::VisitMutWith;

fn emit_error(sp: Span, msg: &str) {
  HANDLER.with(|h| {
    h.struct_span_err(sp, msg).emit();
  });
}

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
      println!("add import");
    }
  }
}

impl TransformVisitor {
  fn v_class_expr(&mut self, n: &mut ClassExpr) {
    if !matches!(&n.class.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
    {
      return;
    }
    self.v_class(n.ident.as_ref(), &mut n.class);
  }
  fn v_class_decl(&mut self, n: &mut ClassDecl) {
    if !matches!(&n.class.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
    {
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
    let mut visitor = JSXVisitor::new();
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

enum Parent {
  Component,
  Html(Html),
  Null,
}
struct Html {
  is_svg: bool,
}

struct Context {
  parent: Parent,
  exprs: Box<Vec<Box<Expr>>>,
}
struct JSXVisitor {
  context: Context,
  stack: Vec<Context>,
}

impl JSXVisitor {
  fn new() -> Self {
    let root_context = Context {
      parent: Parent::Null,
      exprs: Box::new(vec![]),
    };
    Self {
      context: root_context,
      stack: vec![],
    }
  }
  fn push_context(&mut self, p: Parent) {
    let current_context = std::mem::replace(
      &mut self.context,
      Context {
        parent: p,
        exprs: Box::new(vec![]),
      },
    );
    self.stack.push(current_context);
  }
  fn pop_context(&mut self) -> Context {
    std::mem::replace(&mut self.context, self.stack.pop().unwrap())
  }
}

impl Visit for JSXVisitor {
  fn visit_lit(&mut self, n: &Lit) {
    if let Parent::Html(_) = &self.context.parent {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.context.exprs.push(Box::new(e));
    } else {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.context.exprs.push(gen_text_render_func(Box::new(e)));
    }
  }
  fn visit_jsx_text(&mut self, n: &JSXText) {
    let text = n.value.trim();
    if !text.is_empty() {
      self.context.exprs.push(ast_create_expr_lit_str(text));
    }
  }

  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {
        let JSXElementName::Ident(tn) = &n.opening.name else {
          emit_error(n.opening.name.span(), "todo");
          return;
        };
        let tag = tn.as_ref();
        match tag.chars().next() {
          Some(c) if c >= 'A' && c <= 'Z' => {}
          Some(c) if c >= 'a' && c <= 'z' => {
            let mut a_ref: Option<Atom> = None;
            let mut a_lits: Vec<(Ident, Lit)> = vec![];
            n.opening.attrs.iter().for_each(|attr| match attr {
              JSXAttrOrSpread::SpreadElement(s) => {
                emit_error(s.span(), "暂不支持 ... 属性");
              }
              JSXAttrOrSpread::JSXAttr(attr) => {
                let JSXAttrName::Ident(n) = &attr.name else {
                  return;
                };
                let name = &n.sym;
                if name == "ref" {
                  if a_ref.is_some() {
                    emit_error(attr.span(), "不能重复指定 ref");
                    return;
                  }
                  a_ref.replace(name.clone());
                } else if name.starts_with("on")
                  && matches!(name.chars().nth(2), Some(c) if c >= 'A' && c <= 'Z')
                {
                  // html event
                } else {
                  if let Some(val) = &attr.value {
                    match val {
                      JSXAttrValue::Lit(val) => {
                        a_lits.push((tn.clone(), val.clone()));
                      }
                      JSXAttrValue::JSXExprContainer(val) => match &val.expr {
                        JSXExpr::JSXEmptyExpr(_) => {
                          emit_error(val.expr.span(), "属性值为空");
                        }
                        JSXExpr::Expr(expr) => match expr.as_ref() {
                          Expr::JSXElement(_)
                          | Expr::JSXEmpty(_)
                          | Expr::JSXFragment(_)
                          | Expr::JSXMember(_)
                          | Expr::JSXNamespacedName(_) => {
                            emit_error(val.expr.span(), "不支持 JSX 元素作为属性值");
                          }
                          Expr::Lit(val) => {
                            a_lits.push((tn.clone(), val.clone()));
                          }
                          _ => {
                            // expr attribute
                          }
                        },
                      },
                      _ => emit_error(val.span(), "不支持该类型的属性值。"),
                    }
                  } else {
                    // bool attribute
                    a_lits.push((tn.clone(), Lit::Bool(Bool::from(true))));
                  }
                }
              }
            });

            let is_svg = tag == "svg";
            self.push_context(Parent::Html(Html { is_svg }));
            n.visit_children_with(self);
            let context = self.pop_context();
            let mut args = vec![ExprOrSpread {
              spread: None,
              expr: ast_create_expr_lit_str(tag),
            }];
            if !context.exprs.is_empty() {
              args.append(
                &mut context
                  .exprs
                  .into_iter()
                  .map(|expr| ExprOrSpread { spread: None, expr })
                  .collect::<Vec<ExprOrSpread>>(),
              );
            }
            self.context.exprs.push(ast_create_expr_call(
              ast_create_expr_ident(JINGE_IMPORT_CREATE_ELE.1),
              args,
            ));
          }
          _ => {
            emit_error(
              tn.span(),
              "不支持的 Tag。合法 Tag 为：大写字母打头为 Component 组件，小写字母打头为 html 元素。",
            );
            return;
          }
        }
      }
      Expr::JSXEmpty(_) => (),
      Expr::JSXFragment(n) => {
        if n.children.is_empty() {
          return;
        }
      }
      Expr::JSXMember(n) => {
        emit_error(n.span(), "todo");
      }
      Expr::JSXNamespacedName(n) => {
        emit_error(n.span(), "todo");
      }
      Expr::Call(n) => emit_error(n.span(), "不支持函数调用"),
      Expr::Cond(_) => {
        emit_error(n.span(), "不支持二元条件表达式，请使用 <If> 组件");
      }
      Expr::Bin(b) => match b.op {
        BinaryOp::Add
        | BinaryOp::Exp
        | BinaryOp::Sub
        | BinaryOp::Mul
        | BinaryOp::LShift
        | BinaryOp::RShift
        | BinaryOp::ZeroFillRShift
        | BinaryOp::Mod
        | BinaryOp::Div
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor => b.visit_children_with(self),
        _ => emit_error(b.span(), "不支持条件表达式，请使用 <If> 组件"),
      },
      _ => {
        n.visit_children_with(self);
      }
    }
  }
}

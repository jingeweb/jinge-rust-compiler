use std::borrow::Borrow;
use std::ops::Deref;

use enumset::{EnumSet, EnumSetType};
use swc_core::atoms::Atom;
use swc_core::common::{Span, Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Fold, Visit, VisitAll, VisitMut, VisitWith};
use swc_core::plugin::errors::HANDLER;

use crate::ast::{
  ast_create_console_log, ast_create_expr_ident, ast_create_expr_lit_str, ast_create_jinge_import,
};
use crate::common::ImportId;
use crate::config::Config;
use crate::tpl;
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
  pub imports: EnumSet<ImportId>,
}
impl TransformVisitor {
  pub fn new() -> Self {
    Self {
      imports: EnumSet::new(),
    }
  }
}
impl VisitMut for TransformVisitor {
  // Implement necessary visit_mut_* methods for actual custom transform.
  // A comprehensive list of possible visitor methods can be found here:
  // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html

  // fn visit_mut_call_expr(&mut self, n: &mut swc_core::ecma::ast::CallExpr) {}

  // fn visit_mut_module_items(&mut self, n: &mut std::vec::Vec<ModuleItem>) {
  //   println!("{:?}", n.len());
  //   let mut items = Vec::with_capacity(n.len() + 1);
  //   items.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
  //     span: DUMMY_SP,
  //     kind: VarDeclKind::Const,
  //     declare: false,
  //     decls: vec![VarDeclarator {
  //       span: DUMMY_SP,
  //       name: Pat::Ident(BindingIdent {
  //         id: Ident {
  //           span: DUMMY_SP,
  //           sym: Atom::from("xxoo"),
  //           optional: false,
  //         },
  //         type_ann: None,
  //       }),
  //       init: Some(Box::new(Expr::Lit(Lit::Num(Number {
  //         span: DUMMY_SP,
  //         value: 45.0,
  //         raw: None,
  //       })))),
  //       definite: false,
  //     }],
  //   })))));
  //   items.append(n);
  //   *n = items;
  // }
  fn visit_mut_module(&mut self, n: &mut Module) {
    n.visit_mut_children_with(self);

    // {
    //   let mut new_items = Vec::with_capacity(n.body.len() + 1);
    //   new_items.push(ast_create_jinge_import());

    //   new_items.append(&mut n.body);
    //   new_items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    //     span: DUMMY_SP,
    //     expr: Box::new(Expr::Call(CallExpr {
    //       span: DUMMY_SP,
    //       callee: Callee::Expr(ast_create_expr_ident("jinge$textRenderFn$")),
    //       args: vec![],
    //       type_args: None,
    //     })),
    //   })));
    //   n.body = new_items;
    //   println!("xxxx {}", n.body.len());
    //   return;
    // }
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

    if self.imports.len() > 0 {
      let mut new_items = Vec::with_capacity(n.body.len() + 1);
      let imports: Vec<ImportId> = self.imports.iter().collect();
      new_items.push(ast_create_jinge_import(imports));
      // let mut x = n.body.remove(0);
      // println!("{:?}", x);
      // // match &mut x {
      // //   ModuleItem::ModuleDecl(ref mut decl) => match decl {
      // //     ModuleDecl::Import(ref mut imp) => {
      // //       let spec = imp.specifiers.remove(0);
      // //       match spec {
      // //         ImportSpecifier::Named(mut spec) => {
      // //           spec.local = Ident {
      // //             span: spec.local.span(),
      // //             sym: Atom::from("textRenderFn"),
      // //             optional: false,
      // //           }
      // //         }
      // //         _ => (),
      // //       }
      // //     }
      // //     _ => (),
      // //   },
      // //   _ => (),
      // // };
      // // println!("{:?}", x);

      // new_items.push(x);
      // let x = n.body.remove(0);
      // // // println!("{:?}", x);
      // new_items.push(x);
      new_items.append(&mut n.body);
      // new_items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      //   span: DUMMY_SP,
      //   expr: Box::new(Expr::Call(CallExpr {
      //     span: DUMMY_SP,
      //     callee: Callee::Expr(Box::new(Expr::Ident(Ident {
      //       span: DUMMY_SP,
      //       sym: Atom::from("textRenderFn"),
      //       optional: false,
      //     }))),
      //     args: vec![],
      //     type_args: None,
      //   })),
      // })));
      // println!("{:?}", new_items[0]);
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
    let mut visitor = JSXVisitor::new(&mut self.imports);
    visitor.visit_expr(&*return_arg);
    if !visitor.exprs.is_empty() {
      println!("gen render");
      let elems: Vec<Option<ExprOrSpread>> = visitor
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

struct JSXVisitor<'a> {
  imports: &'a mut EnumSet<ImportId>,
  parent: Parent,

  exprs: Vec<Box<Expr>>,
}

impl<'a> JSXVisitor<'a> {
  fn new(imports: &'a mut EnumSet<ImportId>) -> Self {
    Self {
      imports,
      parent: Parent::Null,

      exprs: vec![],
    }
  }
  fn add_import(&mut self, i: ImportId) {
    if !self.imports.contains(i) {
      self.imports.insert(i);
    }
  }
}

impl Visit for JSXVisitor<'_> {
  fn visit_lit(&mut self, n: &Lit) {
    if let Parent::Html(_) = &self.parent {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.exprs.push(Box::new(e));
    } else {
      self.add_import(ImportId::TextRenderFn);
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.exprs.push(tpl::text_render_func(Box::new(e)));
    }
  }
  fn visit_jsx_text(&mut self, n: &JSXText) {
    let text = n.value.trim();
    if !text.is_empty() {
      self.exprs.push(ast_create_expr_lit_str(text));
    }
  }
  // fn visit_jsx_element_child(&mut self, n: &JSXElementChild) {
  //   match n {
  //     JSXElementChild::JSXText(n) => {

  //     }
  //     JSXElementChild::JSXExprContainer(expr) => match &expr.expr {
  //       JSXExpr::JSXEmptyExpr(_) => (),
  //       JSXExpr::Expr(expr) => {
  //         self.visit_expr(expr);
  //       }
  //     },
  //     JSXElementChild::JSXSpreadChild(n) => emit_error(n.span(), "不支持该语法"),
  //     JSXElementChild::JSXElement(c) => {
  //       self.v_jsx_element(c);
  //     }
  //     JSXElementChild::JSXFragment(f) => self.v_jsx_fragment(f),
  //   }
  // }

  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {}
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
        | BinaryOp::BitXor => {}
        _ => emit_error(b.span(), "不支持条件表达式，请使用 <If> 组件"),
      },
      _ => {
        n.visit_children_with(self);
      }
    }
  }
}

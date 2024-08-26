use std::ops::Deref;

use swc_common::DUMMY_SP;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;

use crate::ast::{
  ast_create_arg_expr, ast_create_constructor, ast_create_expr_call, ast_create_expr_call_super,
  ast_create_expr_ident, ast_create_expr_lit_str, ast_create_expr_this,
};
use crate::common::{JINGE_IMPORT_BIND_VM_PARENT, JINGE_IMPORT_MODULE_ITEM, JINGE_RENDER};
use crate::parser;

pub struct TransformVisitor {
  // pub cwd: String,
  // pub filename: String,
  // pub config: Config,
  changed: bool,
}
impl TransformVisitor {
  pub fn new() -> Self {
    // println!("new transform visitor");
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
      new_items.push(JINGE_IMPORT_MODULE_ITEM.clone());
      new_items.append(&mut n.body);

      n.body = new_items;
    }
  }
}

fn is_component(n: &Class) -> bool {
  matches!(&n.super_class, Some(s) if matches!(s.deref(), Expr::Ident(x) if x.sym.as_str() == "Component"))
}

fn bind_inited_props(cont: &mut Constructor, props: Vec<Atom>) {
  let Some(body) = &mut cont.body else {
    return;
  };
  props.into_iter().for_each(|prop| {
    body.stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_BIND_VM_PARENT.local()),
        vec![
          ast_create_arg_expr(ast_create_expr_this()),
          ast_create_arg_expr(ast_create_expr_lit_str(prop)),
        ],
      ),
    }))
  })
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

  fn v_class(&mut self, _ident: Option<&Ident>, class: &mut Class) {
    /*
     * ES 最新的 class 可以在声明属性时直接初始化赋值，这种赋值是直接赋值到原始实例上，而不是经过 vm 包裹后的 Proxy，
     * 因而无法绑定 vm 的父子关系，发生数据变更后无法向上传递。
     *
     * 编译器识别到这种属性时，会在 constructor() 尾部调用该绑定函数，从而建立正确的 vm 关系。
     *
     * 比如：
     * ```tsx
     * import { Component, vm } from 'jinge';
     * class App extends Component {
     *   arr = vm([1, 2, 3]);
     *   render() {
     *     return <div>{this.arr.length}</div>;
     *   }
     * }
     * ```
     * 会被转换成：
     * ```tsx
     * import { Component, vm, bindInitedClassMemberVmParent } from 'jinge';
     * class App extends Component {
     *   arr = vm([1, 2, 3]);
     *   constructor() {
     *     super();
     *     bindInitedClassMemberVmParent(this, 'arr');
     *   }
     *   render() {
     *     return <div>{this.arr.length}</div>;
     *   }
     * }
     */
    let mut render = None;
    let mut public_inited_props = vec![];
    let mut constructor = None;
    if !class
      .body
      .iter()
      .any(|it| matches!(it, ClassMember::Constructor(_)))
    {
      class
        .body
        .push(ClassMember::Constructor(ast_create_constructor(
          vec![],
          vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: ast_create_expr_call_super(vec![]),
          })],
        )));
    }
    class.body.iter_mut().for_each(|it| match it {
      ClassMember::Constructor(c) => {
        constructor = Some(c);
      }
      ClassMember::ClassProp(prop) => {
        // 如果是有初始赋值的成员属性，且不是 _ 打头的单向绑定属性，且不是常量，则需要添加 bindInitedClassMemberVmParent
        if let PropName::Ident(key) = &prop.key {
          if key.sym.starts_with('_') {
            return;
          }
          let Some(v) = &prop.value else {
            return;
          };
          match v.as_ref() {
            Expr::Lit(_) => {}
            _ => {
              public_inited_props.push(key.sym.clone());
            }
          }
        }
      }
      ClassMember::Method(m) => {
        if matches!(&m.key, PropName::Ident(id) if JINGE_RENDER.eq(&id.sym)) {
          render = Some(m.function.as_mut())
        }
      }
      _ => (),
    });

    if !public_inited_props.is_empty() {
      if let Some(cont) = constructor {
        bind_inited_props(cont, public_inited_props);
      }
    }

    let Some(render_fn) = render else {
      // 如果没有 render 函数直接返回。
      return;
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
    if let Some(replaced_expr) = visitor.parse(&*return_arg) {
      return_expr.arg.replace(replaced_expr);
      self.changed = true;
    }
  }
}

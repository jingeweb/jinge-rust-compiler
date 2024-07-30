use crate::ast::{
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_assign_mem, ast_create_expr_call,
  ast_create_expr_ident, ast_create_expr_lit_bool, ast_create_expr_lit_str,
  ast_create_expr_lit_string, ast_create_expr_member, ast_create_expr_this,
  ast_create_id_of_container, ast_create_stmt_decl_const,
};
use crate::common::*;
use swc_core::common::util::take::Take;
use swc_core::common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use tpl::{
  tpl_lit_obj, tpl_push_el_code, tpl_set_attribute, tpl_set_idl_attribute, tpl_set_ref_code,
};

mod attrs;
mod expr;
mod tpl;

#[derive(Debug)]
enum Parent {
  Root,
  Component,
  Html,
  Svg,
}

struct Context {
  // container_component_level: usize,
  root_container: bool,
  parent: Parent,
  expressions: Box<Vec<ExprOrSpread>>,
}

impl Context {
  #[inline]
  pub fn is_parent_svg(&self) -> bool {
    matches!(self.parent, Parent::Svg)
  }
  #[inline]
  pub fn is_parent_component(&self) -> bool {
    matches!(self.parent, Parent::Component)
  }

  #[inline]
  pub fn is_parent_html(&self) -> bool {
    matches!(self.parent, Parent::Html)
  }
}

pub struct TemplateParser {
  context: Context,
  stack: Vec<Context>,
}

impl TemplateParser {
  pub fn new() -> Self {
    let root_context = Context {
      // container_component_level: 0,
      root_container: true,
      parent: Parent::Component,
      expressions: Box::new(vec![]),
    };
    Self {
      context: root_context,
      stack: vec![],
    }
  }
  fn push_context(&mut self, parent: Parent, root_container: bool) {
    // let container_component_level = self.context.container_component_level;
    let current_context = std::mem::replace(
      &mut self.context,
      Context {
        // container_component_level: if inc_container_component_level {
        //   container_component_level + 1
        // } else {
        //   container_component_level
        // },
        root_container,
        parent,
        expressions: Box::new(vec![]),
      },
    );
    self.stack.push(current_context);
  }
  fn pop_context(&mut self) -> Context {
    std::mem::replace(&mut self.context, self.stack.pop().unwrap())
  }
  pub fn parse(&mut self, expr: &Expr) -> Option<Box<Expr>> {
    self.visit_expr(expr);
    let elems: Vec<Option<ExprOrSpread>> = self
      .context
      .expressions
      .take()
      .into_iter()
      .map(|e| Some(e))
      .collect();
    if elems.is_empty() {
      None
    } else {
      Some(Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems,
      })))
    }
  }

  fn parse_html_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, false);
    self.push_context(
      if tn.as_ref() == "svg" {
        Parent::Svg
      } else {
        Parent::Html
      },
      self.context.root_container,
    );
    // println!("meet html {} {}", tn.sym.as_str(), self.context.slot_level);
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let root_container = self.context.root_container;
    let mut children_context = self.pop_context();
    let callee_ident = if self.context.is_parent_svg() || tn.sym.eq("svg") {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_ELE.local()
      }
    } else {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_ELE.local()
      }
    };
    let mut args = vec![ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(
      Str::from(tn.sym.clone()),
    ))))];
    let set_ref_code = attrs.ref_prop.take().map(|r| tpl_set_ref_code(r));
    let push_ele_code = if self.context.is_parent_component() {
      Some(tpl_push_el_code(true, root_container))
    } else {
      None
    };
    if !attrs.const_props.is_empty() {
      args.push(ast_create_arg_expr(tpl_lit_obj(attrs.const_props)));
    }
    if !children_context.expressions.is_empty() {
      args.append(&mut children_context.expressions);
    }

    let output = if set_ref_code.is_some()
      || push_ele_code.is_some()
      || !attrs.evt_props.is_empty()
      || !attrs.watch_props.is_empty()
    {
      let mut stmts: Vec<Stmt> = vec![ast_create_stmt_decl_const(
        JINGE_EL_IDENT.clone(),
        ast_create_expr_call(ast_create_expr_ident(callee_ident), args),
      )];
      attrs.evt_props.into_iter().for_each(|evt| {
        let mut args = vec![
          ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
          ast_create_arg_expr(ast_create_expr_lit_string(evt.event_name)),
          ast_create_arg_expr(evt.event_handler),
        ];
        if evt.capture {
          args.push(ast_create_arg_expr(ast_create_expr_lit_bool(true)));
        }
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_ADD_EVENT.local()), args),
        }))
      });
      attrs
        .watch_props
        .into_iter()
        .for_each(|(attr_name, watch_expr)| {
          let set_expr = if IDL_ATTRIBUTE_SET.binary_search(&attr_name.sym).is_ok() {
            tpl_set_idl_attribute(
              ast_create_expr_ident(JINGE_EL_IDENT.clone()),
              attr_name.sym,
              ast_create_expr_ident(JINGE_V_IDENT.clone()),
            )
          } else {
            tpl_set_attribute(
              ast_create_expr_ident(JINGE_EL_IDENT.clone()),
              attr_name.sym,
              ast_create_expr_ident(JINGE_V_IDENT.clone()),
            )
          };
          let args = vec![
            ast_create_arg_expr(ast_create_expr_this()),
            ast_create_arg_expr(watch_expr),
            ast_create_arg_expr(ast_create_expr_arrow_fn(
              vec![Pat::Ident(BindingIdent::from(JINGE_V_IDENT.clone()))],
              Box::new(BlockStmtOrExpr::Expr(set_expr)),
            )),
          ];
          stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: ast_create_expr_call(
              ast_create_expr_ident(JINGE_IMPORT_WATCH_FOR_COMPONENT.local()),
              args,
            ),
          }));
        });
      if let Some(c) = set_ref_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      if let Some(c) = push_ele_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      }));
      let body = Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
        ctxt: SyntaxContext::empty(),
        span: DUMMY_SP,
        stmts,
      }));
      let callee = Box::new(Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(Expr::Arrow(ArrowExpr {
          ctxt: SyntaxContext::empty(),
          span: DUMMY_SP,
          params: vec![],
          body,
          is_async: false,
          is_generator: false,
          type_params: None,
          return_type: None,
        })),
      }));
      ast_create_expr_call(callee, vec![])
    } else {
      ast_create_expr_call(ast_create_expr_ident(callee_ident), args)
    };

    self.context.expressions.push(ExprOrSpread {
      spread: None,
      expr: output,
    });
  }
  fn parse_component_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, true);
    self.push_context(Parent::Component, false);
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let children_context = self.pop_context();
    let root_container = self.context.root_container;
    let elems: Vec<_> = children_context
      .expressions
      .into_iter()
      .map(|e| Some(e))
      .collect();

    let mut stmts: Vec<Stmt> = vec![ast_create_stmt_decl_const(
      JINGE_ATTR_IDENT.clone(),
      if !attrs.watch_props.is_empty() {
        ast_create_expr_call(
          ast_create_expr_ident(JINGE_IMPORT_VM.local()),
          vec![ast_create_arg_expr(tpl_lit_obj(attrs.const_props))],
        )
      } else {
        tpl_lit_obj(attrs.const_props)
      },
    )];
    attrs
      .watch_props
      .into_iter()
      .for_each(|(attr_name, watch_expr)| {
        let x = ast_create_expr_assign_mem(
          MemberExpr {
            span: DUMMY_SP,
            obj: ast_create_expr_ident(JINGE_ATTR_IDENT.clone()),
            prop: MemberProp::Ident(attr_name),
          },
          ast_create_expr_ident(JINGE_V_IDENT.clone()),
        );
        let set_expr = ast_create_expr_arrow_fn(
          vec![Pat::Ident(BindingIdent::from(JINGE_V_IDENT.clone()))],
          Box::new(BlockStmtOrExpr::Expr(x)),
        );
        let args = vec![
          ast_create_arg_expr(ast_create_expr_this()),
          ast_create_arg_expr(watch_expr),
          ast_create_arg_expr(set_expr),
        ];
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(
            ast_create_expr_ident(JINGE_IMPORT_WATCH_FOR_COMPONENT.local()),
            args,
          ),
        }));
      });

    let set_ref_code = attrs.ref_prop.take().map(|r| tpl_set_ref_code(r));
    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_NEW_COM_DEFAULT_SLOT.local()),
        vec![
          ast_create_arg_expr(Box::new(Expr::Ident(Ident::from(tn.sym.clone())))),
          ast_create_arg_expr(ast_create_expr_ident(JINGE_ATTR_IDENT.clone())),
          ast_create_arg_expr(ast_create_expr_arrow_fn(
            vec![Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone()))],
            Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
              span: DUMMY_SP,
              elems,
            })))),
          )),
        ],
      ),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), root_container),
    }));
    if let Some(c) = set_ref_code {
      stmts.push(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: c,
      }))
    }

    stmts.push(Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(ast_create_expr_call(
        ast_create_expr_member(
          ast_create_expr_ident(JINGE_EL_IDENT.clone()),
          MemberProp::Ident(IdentName::from("render")),
        ),
        vec![],
      )),
    }));
    self.context.expressions.push(ExprOrSpread {
      spread: Some(DUMMY_SP),
      expr: ast_create_expr_call(
        ast_create_expr_arrow_fn(
          vec![],
          Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts,
          })),
        ),
        vec![],
      ),
    });
  }
}

impl Visit for TemplateParser {
  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {
        self.visit_jsx_element(&*n);
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

  fn visit_jsx_element(&mut self, n: &JSXElement) {
    let JSXElementName::Ident(tn) = &n.opening.name else {
      emit_error(
        n.opening.name.span(),
        "未知的 JSX 格式，opening.name 未找到",
      );
      return;
    };
    // let tag = tn.as_ref();
    println!("visit jsx ele: {}", tn.as_ref());
    match tn.as_ref().chars().next() {
      Some(c) if c.is_ascii_uppercase() => {
        self.parse_component_element(tn, n);
      }
      Some(c) if c.is_ascii_lowercase() => {
        self.parse_html_element(tn, n);
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
  fn visit_jsx_text(&mut self, n: &JSXText) {
    let text = n.value.trim();
    if text.is_empty() {
      return;
    }
    // println!("sll {} {}", container_component_level, self.context.is_parent_component());
    if !self.context.is_parent_component() {
      self
        .context
        .expressions
        .push(ast_create_arg_expr(ast_create_expr_lit_str(text, None)));
    } else {
      self
        .context
        .expressions
        .push(ast_create_arg_expr(ast_create_expr_call(
          ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.local()),
          vec![
            ast_create_arg_expr(ast_create_id_of_container(self.context.root_container)),
            ast_create_arg_expr(ast_create_expr_lit_str(text, None)),
          ],
        )));
    }
  }

  fn visit_lit(&mut self, n: &Lit) {
    if !self.context.is_parent_component() {
      self
        .context
        .expressions
        .push(ast_create_arg_expr(Box::new(Expr::Lit(n.clone()))));
    } else {
      self
        .context
        .expressions
        .push(ast_create_arg_expr(ast_create_expr_call(
          ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.local()),
          vec![
            ast_create_arg_expr(ast_create_expr_this()),
            ast_create_arg_expr(Box::new(Expr::Lit(n.clone()))),
          ],
        )));
    }
  }
}

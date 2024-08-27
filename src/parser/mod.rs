use crate::ast::{
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
  ast_create_expr_lit_bool, ast_create_expr_lit_str, ast_create_expr_member,
  ast_create_id_of_container, ast_create_stmt_decl_const,
};
use crate::common::*;
use expr::{ExprParseResult, ExprVisitor};
use swc_core::atoms::Atom;
use swc_core::common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use tpl::{
  tpl_lit_obj, tpl_push_el_code, tpl_render_const_text, tpl_render_expr_text, tpl_set_ref_code,
  tpl_watch_and_set_component_attr, tpl_watch_and_set_html_attr,
};

mod attrs;
mod cond;
mod expr;
mod map;
mod slot;
mod tpl;

enum Parent {
  Component,
  Html,
  Svg,
}

struct Slot {
  name: Atom,
  params: Vec<Pat>,
  expressions: Vec<ExprOrSpread>,
}
impl Slot {
  fn new(name: Atom) -> Self {
    Self {
      name,
      params: vec![],
      expressions: vec![],
    }
  }
}
struct Context {
  // container_component_level: usize,
  root_container: bool,
  parent: Parent,
  slots: Vec<Slot>,
}

impl Context {
  fn new(parent: Parent, root_container: bool) -> Self {
    Self {
      root_container,
      parent,
      slots: vec![Slot::new(Atom::default())], // 第 0 个 Slot 是默认 DEFAULT_SLOT
    }
  }
  #[inline]
  pub fn is_parent_svg(&self) -> bool {
    matches!(self.parent, Parent::Svg)
  }
  #[inline]
  pub fn is_parent_component(&self) -> bool {
    matches!(self.parent, Parent::Component)
  }
}

pub struct TemplateParser {
  context: Context,
  stack: Vec<Context>,
}

fn has_jsx(expr: &Expr) -> bool {
  match expr {
    Expr::JSXElement(_) | Expr::JSXFragment(_) => true,
    Expr::Cond(e) => {
      return has_jsx(&e.alt) || has_jsx(&e.cons);
    }
    Expr::Bin(e) => return e.op == BinaryOp::LogicalAnd && has_jsx(&e.right),
    Expr::Paren(e) => return has_jsx(&e.expr),
    _ => {
      return false;
    }
  }
}

impl TemplateParser {
  pub fn new() -> Self {
    Self {
      context: Context::new(Parent::Component, true),
      stack: vec![],
    }
  }
  fn push_context(&mut self, parent: Parent, root_container: bool) {
    let current_context =
      std::mem::replace(&mut self.context, Context::new(parent, root_container));
    self.stack.push(current_context);
  }
  fn pop_context(&mut self) -> Context {
    std::mem::replace(&mut self.context, self.stack.pop().unwrap())
  }
  #[inline]
  /// push expression to last slot
  fn push_expression(&mut self, e: Box<Expr>) {
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ast_create_arg_expr(e));
  }
  pub fn parse(&mut self, expr: &Expr) -> Option<Box<Expr>> {
    if has_jsx(expr) || matches!(expr, Expr::Lit(_)) {
      self.visit_expr(expr);
    } else {
      return None;
    }
    assert_eq!(self.context.slots.len(), 1);
    let elems: Vec<Option<ExprOrSpread>> = self
      .context
      .slots
      .pop()
      .unwrap()
      .expressions
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
      if JINGE_SVG.eq(&tn.sym) {
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
    // html 元素下不可能出现多个 slots。事实上，html 元素没有 slot 概念，只是用统一的数据结构保存子节点。
    assert_eq!(children_context.slots.len(), 1);
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
    if !children_context.slots[0].expressions.is_empty() {
      args.append(&mut children_context.slots[0].expressions);
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
          ast_create_arg_expr(ast_create_expr_lit_str(evt.event_name)),
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
          stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: tpl_watch_and_set_html_attr(attr_name, watch_expr, self.context.root_container),
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
    // 当前 html 元素添加到父亲的最顶部 Slot 中。最顶部 Slot 可能是默认 Slot(比如父亲也是 html 元素则也是存放在默认 Slot)，也可能是命名 Slot(只可能出现在父亲是组件的情况)
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
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
      .for_each(|(attr_name, expr_result)| {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_set_component_attr(
            attr_name,
            expr_result,
            self.context.root_container,
          ),
        }));
      });

    let set_ref_code = attrs.ref_prop.take().map(|r| tpl_set_ref_code(r));
    let mut slots = children_context.slots;
    let mut args = vec![
      ast_create_arg_expr(Box::new(Expr::Ident(Ident::from(tn.sym.clone())))),
      ast_create_arg_expr(ast_create_expr_ident(JINGE_ATTR_IDENT.clone())),
      ast_create_arg_expr(ast_create_expr_member(
        ast_create_id_of_container(root_container),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
        }),
      )),
    ];
    let has_named_slots = slots.len() > 1;
    if has_named_slots {
      assert!(slots[0].expressions.is_empty());
      let x: Vec<_> = slots
        .into_iter()
        .skip(1)
        .filter(|s| !s.expressions.is_empty()) // 跳过默认 DEFAULT_SLOT，一定是空的
        .map(|mut s| {
          let mut params = vec![Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone()))];
          params.append(&mut s.params);
          (
            IdentName::from(s.name),
            ast_create_expr_arrow_fn(
              params,
              Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: s.expressions.into_iter().map(|e| Some(e)).collect(),
              })))),
            ),
          )
        })
        .collect();
      args.push(ast_create_arg_expr(tpl_lit_obj(x)));
    } else {
      let mut default_slot = slots.pop().unwrap();
      if !default_slot.expressions.is_empty() {
        let mut params = vec![Pat::Ident(BindingIdent::from(JINGE_HOST_IDENT.clone()))];
        params.append(&mut default_slot.params);
        args.push(ast_create_arg_expr(ast_create_expr_arrow_fn(
          params,
          Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: default_slot
              .expressions
              .into_iter()
              .map(|e| Some(e))
              .collect(),
          })))),
        )))
      }
    }

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(if has_named_slots {
          JINGE_IMPORT_NEW_COM_SLOTS.local()
        } else {
          JINGE_IMPORT_NEW_COM_DEFAULT_SLOT.local()
        }),
        args,
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
          MemberProp::Ident(IdentName::from(JINGE_RENDER.clone())),
        ),
        vec![],
      )),
    }));
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
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
  fn parse_expr(&mut self, expr: &Expr) {
    let expr_result = ExprVisitor::new().parse(expr);
    // println!("{:#?}", expr);
    match expr_result {
      ExprParseResult::None => self.push_expression(tpl_render_const_text(
        Box::new(expr.clone()),
        self.context.is_parent_component(),
        self.context.root_container,
      )),
      _ => {
        self.push_expression(tpl_render_expr_text(
          expr_result,
          ast_create_expr_ident(JINGE_V_IDENT.clone()),
          self.context.is_parent_component(),
          self.context.root_container,
        ));
      }
    }
  }
}

impl Visit for TemplateParser {
  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {
        self.visit_jsx_element(&*n);
      }
      Expr::JSXEmpty(_) => (),
      Expr::JSXFragment(f) => {
        f.visit_children_with(self);
      }
      Expr::JSXMember(_) | Expr::JSXNamespacedName(_) => emit_error(n.span(), "不支持的 jsx 格式"),
      Expr::Call(expr) => {
        if self.parse_map_fn(expr) {
          // 如果是 [xx].map() 函数调用，则转换为 <For> 组件。
        } else if self.parse_slot_call_expr(expr) {
          // 如果是 this.slots() 或 this.slots.xx() 的调用，则转换为 Slot
        } else {
          // 其它情况当成通用表达式进行转换。
          self.parse_expr(n);
        }
      }
      Expr::Cond(e) => {
        self.parse_cond_expr(e);
      }
      Expr::Bin(e) => {
        if e.op == BinaryOp::LogicalAnd {
          self.parse_logic_and_expr(e);
        } else {
          self.parse_expr(n);
        }
      }

      Expr::Fn(f) => {
        emit_error(f.span(), "tsx 中不支持函数，如果是定义 Slot 请使用箭头函数");
      }
      Expr::Arrow(expr) => {
        if !self.context.is_parent_component() || self.context.root_container {
          emit_error(expr.span(), "Slot 定义必须位于组件下");
          return;
        }
        match &*expr.body {
          BlockStmtOrExpr::BlockStmt(_) => {
            emit_error(
              expr.span(),
              "使用箭头函数定义默认 Slot 时必须直接在箭头后返回值",
            );
          }
          BlockStmtOrExpr::Expr(e) => {
            if !expr.params.is_empty() {
              self
                .context
                .slots
                .last_mut()
                .unwrap()
                .params
                .append(&mut expr.params.clone());
            }
            // println!("{:#?}", e);
            self.visit_expr(e);
            // e.as_ref().visit_children_with(self);
          }
        }
      }
      Expr::Object(obj) => {
        if !self.context.is_parent_component() || self.context.root_container {
          emit_error(obj.span(), "Slot 定义必须位于组件下");
          return;
        }
        obj.props.iter().for_each(|prop| match prop {
          PropOrSpread::Spread(e) => {
            emit_error(e.dot3_token.span(), "Slot 定义不支持 ... 的书写方式");
          }
          PropOrSpread::Prop(p) => match p.as_ref() {
            Prop::KeyValue(KeyValueProp { key, value }) => {
              match key {
                PropName::Ident(id) => self.context.slots.push(Slot::new(id.sym.clone())),
                PropName::Str(s) => self.context.slots.push(Slot::new(s.value.clone())),
                _ => {
                  emit_error(key.span(), "Slot 定义的名称必须是常量字符串");
                  return;
                }
              }
              self.visit_expr(value);
            }
            _ => emit_error(p.span(), "Slot 定义必须是 Key: Value 的形式"),
          },
        })
      }
      Expr::Array(e) => emit_error(e.span(), "tsx 中不能直接使用数组表达式"),
      Expr::Paren(e) => self.visit_expr(&e.expr),
      Expr::Lit(e) => self.visit_lit(e),
      _ => self.parse_expr(n),
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
    // println!("visit jsx ele: {}", tn.as_ref());
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

  fn visit_jsx_expr(&mut self, node: &JSXExpr) {
    if let JSXExpr::Expr(expr) = node {
      self.visit_expr(expr.as_ref());
    }
  }
  fn visit_jsx_text(&mut self, t: &JSXText) {
    let t = t.value.trim();
    if t.is_empty() {
      return;
    }
    self.push_expression(tpl_render_const_text(
      ast_create_expr_lit_str(Atom::from(t)),
      self.context.is_parent_component(),
      self.context.root_container,
    ))
  }
  fn visit_lit(&mut self, n: &Lit) {
    if let Lit::JSXText(t) = n {
      self.visit_jsx_text(t);
    } else {
      self.push_expression(tpl_render_const_text(
        Box::new(Expr::Lit(n.clone())),
        self.context.is_parent_component(),
        self.context.root_container,
      ))
    };
  }
}

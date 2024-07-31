use crate::ast::{
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_assign_mem, ast_create_expr_call,
  ast_create_expr_ident, ast_create_expr_lit_bool, ast_create_expr_lit_str,
  ast_create_expr_lit_string, ast_create_expr_member, ast_create_expr_this,
  ast_create_id_of_container, ast_create_stmt_decl_const,
};
use crate::common::*;
use swc_core::atoms::Atom;
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

struct Slot {
  name: Option<Atom>,
  params: Vec<Pat>,
  expressions: Vec<ExprOrSpread>,
}
impl Slot {
  fn new() -> Self {
    Self {
      name: None, // None 代表默认 DEFAULT_SLOT
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
      slots: vec![Slot::new()], // 第 0 个 Slot 是默认 DEFAULT_SLOT
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
  fn visit_jsx_expr(&mut self, node: &JSXExpr) {
    if let JSXExpr::Expr(expr) = node {
      self.visit_expr(expr.as_ref());
    }
  }
  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {
        self.visit_jsx_element(&*n);
      }
      Expr::Call(expr) => {
        // 如果是 this.props.children() 或 this.props.children.xx() 的调用，则转换为 Slot
      }
      Expr::Cond(_) => {
        emit_error(n.span(), "不支持二元条件表达式，请使用 <If> 组件");
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
              self.context.slots[0]
                .params
                .append(&mut expr.params.clone());
            }
            e.as_ref().visit_children_with(self);
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
                PropName::Ident(id) => {}
                PropName::Str(s) => {}
                _ => {
                  emit_error(key.span(), "Slot 定义的名称必须是常量字符串");
                  return;
                }
              }
              match value.as_ref() {
                Expr::Arrow(expr) => {}
                _ => emit_error(value.span(), "定义指定名称 Slot 时，值必须是箭头函数"),
              }
            }
            _ => emit_error(p.span(), "Slot 定义必须是 Key: Value 的形式"),
          },
        })
      }
      Expr::Array(e) => emit_error(e.span(), "tsx 中不能直接使用数组表达式"),
      _ => {
        emit_error(n.span(), "tsx 中不支持该表达式");
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

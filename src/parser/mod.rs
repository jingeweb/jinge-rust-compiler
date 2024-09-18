use crate::ast::*;
use crate::common::*;
use expr::{ExprParseResult, ExprVisitor};
use swc_core::atoms::Atom;
use swc_core::common::{Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use tpl::*;

mod attrs;
mod component;
mod cond;
mod expr;
mod jsx;
mod map;
mod map_key;
mod slot;
mod tpl;

pub enum Parent {
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
  props_arg: Option<Atom>,
  map_loop_level: usize,
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
  pub fn new(props_arg: Option<Atom>) -> Self {
    Self {
      context: Context::new(Parent::Component, true),
      stack: vec![],
      props_arg,
      map_loop_level: 0,
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
  fn parse_mem(&mut self, parent_expr: &Expr, expr: &MemberExpr) {
    if !self.parse_slot_mem_expr(expr, None) {
      self.parse_expr(parent_expr);
    }
  }
  fn parse_call(&mut self, parent_expr: &Expr, callee: &Expr, args: &Vec<ExprOrSpread>) {
    if self.parse_map_fn(callee, args) {
      // 如果是 [xx].map() 函数调用，则转换为 <For> 组件。
    } else if self.parse_slot_call_expr(callee, args) {
      // 如果是 props.children() 或 props.children.xx() 的调用，则转换为 Slot
    } else {
      // 其它情况当成通用表达式进行转换。
      self.parse_expr(parent_expr);
    }
  }
}

impl Visit for TemplateParser {
  fn visit_jsx_element(&mut self, n: &JSXElement) {
    self.parse_jsx_element(n);
  }
  fn visit_expr(&mut self, expr_node: &Expr) {
    match expr_node {
      Expr::JSXElement(n) => {
        self.visit_jsx_element(&*n);
      }
      Expr::JSXEmpty(_) => (),
      Expr::JSXFragment(f) => {
        f.visit_children_with(self);
      }
      Expr::JSXMember(_) | Expr::JSXNamespacedName(_) => {
        emit_error(expr_node.span(), "不支持的 jsx 格式")
      }

      Expr::Cond(e) => {
        self.parse_cond_expr(e);
      }
      Expr::Bin(e) => {
        if e.op == BinaryOp::LogicalAnd {
          self.parse_logic_and_expr(e);
        } else {
          self.parse_expr(expr_node);
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
              expr.params.iter().any(|par| {
                if !matches!(par, Pat::Ident(_)) {
                  emit_error(
                    par.span(),
                    "警告：slot 函数的参数不要使用解构的写法，会导致数据的绑定失效。",
                  );
                  true
                } else {
                  false
                }
              });
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
      Expr::Call(expr) => {
        if let Callee::Expr(callee) = &expr.callee {
          self.parse_call(expr_node, callee.as_ref(), &expr.args);
        }
      }
      Expr::Member(e) => self.parse_mem(expr_node, e),
      Expr::OptChain(opt) => match opt.base.as_ref() {
        OptChainBase::Member(e) => self.parse_mem(expr_node, e),
        OptChainBase::Call(c) => {
          self.parse_call(expr_node, c.callee.as_ref(), &c.args);
        }
      },
      _ => self.parse_expr(expr_node),
    }
  }
  fn visit_jsx_expr(&mut self, node: &JSXExpr) {
    if let JSXExpr::Expr(expr) = node {
      self.visit_expr(expr.as_ref());
    }
  }
  fn visit_jsx_text(&mut self, text_node: &JSXText) {
    let text = &text_node.value;
    if text.is_empty() {
      return;
    }
    let Some(text) = trim_html_text(text) else {
      return;
    };
    self.push_expression(tpl_render_const_text(
      ast_create_expr_lit_str(text),
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

/**
 * 处理 jsx text 中的空白（\n,\t 和空格），和 react 保持一致。
 * 首尾的空白，如果包含了 \n，则全部 trim 去除；否则全部保留；
 * 中间的空白，如果包含了 \n，替换为单个空格；否则全部保留。
 */
fn trim_html_text(text: &Atom) -> Option<Atom> {
  let mut result = String::new();

  let mut start_i = 0i32;
  let mut not_whitespace_i = -1;
  let mut meet_break_line = false;
  let mut break_line_i = -1i32;
  let mut meet_not_whitespace = false;

  let bytes = text.as_bytes();
  for i in 0..bytes.len() {
    let chr = bytes[i];
    if chr == b'\n' {
      meet_break_line = true;
      break_line_i = i as i32;
      if not_whitespace_i >= 0 {
        result.push_str(&text[(start_i as usize)..=(not_whitespace_i as usize)]);
      }
      not_whitespace_i = -1;
      start_i = -1;
    } else if chr != b' ' && chr != b'\t' {
      if break_line_i >= 0 {
        if meet_not_whitespace {
          // 位于中间的带 \n 的空白才需要被替换为单个空格。首尾的带 \n 空白直接 trim 去除。
          result.push_str(" ");
        }
        break_line_i = -1;
      }
      meet_not_whitespace = true;
      not_whitespace_i = i as i32;
      if start_i < 0 {
        start_i = i as i32;
      }
    }
  }
  if meet_break_line && not_whitespace_i >= 0 {
    result.push_str(&text[(start_i as usize)..]);
  }

  if !meet_break_line {
    Some(text.clone())
  } else if result.is_empty() {
    None
  } else {
    Some(Atom::from(result))
  }
}

#[test]
fn test_trim_html_text() {
  let mut t = Atom::from("  hello  ");
  assert_eq!(trim_html_text(&t), Some(t));
  t = Atom::from("   \n   hello  ");
  assert_eq!(trim_html_text(&t), Some(Atom::from("hello  ")));
  t = Atom::from("   \n   he  llo  \n   ");
  assert_eq!(trim_html_text(&t), Some(Atom::from("he  llo")));
  t = Atom::from(" he llo \n w \n  orld");
  assert_eq!(trim_html_text(&t), Some(Atom::from(" he llo w orld")));
  t = Atom::from("  \n  \n\n  ");
  assert_eq!(trim_html_text(&t), None);
  t = Atom::from("  a \n\n\n b c \n d ");
  assert_eq!(trim_html_text(&t), Some(Atom::from("  a b c d ")));
}

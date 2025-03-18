use crate::ast::*;
use crate::common::*;
use expr::{ExprParseResult, ExprVisitor};
use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, Spanned};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use tpl::*;

mod attrs;
mod component;
mod cond;
mod expr;
pub mod intl;
mod jsx;
mod map;
mod map_key;
mod slot;
pub mod tpl;

#[derive(Debug, Clone, Copy)]
pub enum Parent {
  Component,
  Html,
  Svg,
}

pub struct Slot {
  name: Atom,
  /// 插槽函数的参数
  params: Vec<Pat>,
  /// 插槽函数的除 return 之外的语句。
  stmts: Vec<Stmt>,
  /// 插槽函数 return 语句转换后的渲染表达式。比如 return <>...</> 里面可能有多个 jsx 元素。
  expressions: Vec<ExprOrSpread>,
}
impl Slot {
  fn new(name: Atom) -> Self {
    Self {
      name,
      params: vec![],
      stmts: vec![],
      expressions: vec![],
    }
  }
}
struct Context {
  // container_component_level: usize,
  // root_container: bool,
  parent: Parent,
  host_ident: Option<Ident>,
  slots: Vec<Slot>,
}

impl Context {
  fn new(parent: Parent) -> Self {
    Self::new_with_host_ident(parent, None)
  }
  fn new_with_host_ident(parent: Parent, host_ident: Option<Ident>) -> Self {
    Self {
      host_ident: host_ident,
      // root_container,
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
  intl_type: IntlType,
  context: Context,
  stack: Vec<Context>,
  /// 组件标签元素（非 html 或 svg 元素）的层级深度。默认为0代表最外层的函数组件，每遇到（进入）一个组件标签 +1，退出 -1
  fc_deep: usize,
  /// 最外层的函数组件的第一个参数，即 Props 属性参数。
  props_arg: Option<Atom>,

  map_loop_level: usize,
}

impl TemplateParser {
  pub fn new(props_arg: Option<Atom>, host_ident: Option<Ident>, intl_type: IntlType) -> Self {
    Self {
      intl_type,
      props_arg,
      context: Context::new_with_host_ident(Parent::Component, host_ident),
      stack: vec![],
      fc_deep: 0,
      map_loop_level: 0,
    }
  }
  pub fn push_context(&mut self, parent: Parent) {
    if matches!(&parent, Parent::Component) {
      self.fc_deep += 1;
    }
    let current_context = std::mem::replace(&mut self.context, Context::new(parent));
    self.stack.push(current_context);
  }
  pub fn push_context_inherit_host_ident(&mut self, parent: Parent) {
    let host_ident = self.context.host_ident.clone();
    let current_context = std::mem::replace(
      &mut self.context,
      Context::new_with_host_ident(parent, host_ident),
    );
    self.stack.push(current_context);
  }
  fn pop_context(&mut self) -> Context {
    let ctx = std::mem::replace(&mut self.context, self.stack.pop().unwrap());
    if matches!(&ctx.parent, Parent::Component) {
      self.fc_deep -= 1;
    }
    ctx
  }
  #[inline]
  fn create_host_ident(&self) -> Ident {
    self.context.host_ident.as_ref().map_or_else(
      || {
        if self.fc_deep > 0 {
          JINGE_HOST_IDENT.clone()
        } else {
          JINGE_ROOT_HOST_IDENT.clone()
        }
      },
      |id| id.clone(),
    )
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
  #[inline]
  /// push spread expression to last slot
  fn push_expression_with_spread(&mut self, e: Box<Expr>) {
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
        spread: Some(DUMMY_SP),
        expr: e,
      });
  }
  pub fn parse(&mut self, expr: &Expr) -> Option<Box<Expr>> {
    self.visit_expr(expr);
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
    let host_ident = self.create_host_ident();

    // println!("{:#?}", expr);
    match expr_result {
      ExprParseResult::None => self.push_expression(tpl_render_const_text(
        Box::new(expr.clone()),
        self.context.is_parent_component(),
        host_ident,
      )),
      _ => {
        self.push_expression(tpl_render_expr_text(
          expr_result,
          ast_create_expr_ident(JINGE_V_IDENT.clone()),
          self.context.is_parent_component(),
          host_ident,
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
    if matches!(self.intl_type, IntlType::Enabled(_)) && self.parse_intl_t(callee, args) {
      // 如果是 t 函数，则转换为国际化组件。
    } else if self.parse_map_fn(callee, args) {
      // 如果是 [xx].map() 函数调用，则转换为 <For> 组件。
    } else if self.parse_slot_call_expr(callee, args) {
      // 如果是 props.children() 或 props.children.xx() 的调用，则转换为 Slot
    } else {
      // 其它情况当成通用表达式进行转换。
      self.parse_expr(parent_expr);
    }
  }
  fn parse_func_function(&mut self, expr: &FnExpr) {
    let Some(body) = &expr.function.body else {
      emit_error(expr.function.span(), "插槽函数必须有返回值");
      return;
    };
    let params: Vec<_> = expr.function.params.iter().map(|p| p.pat.clone()).collect();
    self.parse_func_body(body, &params);
  }
  fn parse_func_arrow(&mut self, expr: &ArrowExpr) {
    match &*expr.body {
      BlockStmtOrExpr::BlockStmt(b) => {
        self.parse_func_body(b, &expr.params);
      }
      BlockStmtOrExpr::Expr(e) => {
        self.parse_func_return(e, &expr.params);
      }
    }
  }
  fn parse_func_body(&mut self, body: &BlockStmt, params: &Vec<Pat>) {
    const ERR: &str = "插槽函数必须有返回值";
    let Some(Stmt::Return(r)) = body.stmts.last() else {
      emit_error(body.span(), ERR);
      return;
    };
    let Some(rtn) = &r.arg else {
      emit_error(body.span(), ERR);
      return;
    };
    self.parse_func_return(rtn, params);

    let len = body.stmts.len();
    for stmt in &body.stmts[0..len - 1] {
      self
        .context
        .slots
        .last_mut()
        .unwrap()
        .stmts
        .push(stmt.clone());
    }
  }
  fn parse_func_return(&mut self, expr: &Box<Expr>, params: &Vec<Pat>) {
    if !self.context.is_parent_component() {
      emit_error(expr.span(), "插槽函数不能定义在 html 元素下");
      return;
    }
    if params.len() > 2 {
      emit_error(
        params[0].span(),
        "插槽函数的参数不能超过3个，第一个是 Props 属性，第二个是 Host 组件。",
      );
      return;
    }
    let props_param;
    if let Some(p) = params.get(0) {
      if !matches!(p, Pat::Ident(_)) {
        emit_error(
          p.span(),
          "插槽函数的第一个参数只能是普通 Ident 格式。不要使用解构一类的写法，会导致数据绑定失效。",
        );
        return;
      } else {
        // slot_params.push(p.clone());
        props_param = p.clone();
      }
    } else {
      props_param = Pat::Ident(BindingIdent::from(JINGE_ATTR_IDENT.clone()));
    }
    let host_param;
    if let Some(p) = params.get(1) {
      if let Pat::Ident(id) = p {
        if !id.id.sym.starts_with("host") {
          emit_error(
            id.span(),
            "插槽函数的第二个参数名必须是 host 或以 host 打头，确保已对第二个参数有充分理解",
          );
          return;
        } else {
          host_param = id.clone();
        }
      } else {
        emit_error(p.span(), "插槽函数的第二个参数只能是普通 Ident 格式。");
        return;
      }
    } else {
      host_param = BindingIdent::from(JINGE_HOST_IDENT.clone());
    }

    let slot = self.context.slots.last_mut().unwrap();
    let slot_params = &mut slot.params;
    self.context.host_ident.replace(host_param.id.clone());
    slot_params.push(props_param);
    slot_params.push(Pat::Ident(host_param));
    self.visit_expr(expr);
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
        if !self.parse_cond_expr(e) {
          self.parse_expr(expr_node);
        }
      }
      Expr::Bin(e) => {
        // 处理在 tsx 中常见的 binary 表达式，比如 `someVar && <div>hello</div>`，`someVar ?? <div>hello</div>`
        if e.op == BinaryOp::LogicalAnd {
          if !self.parse_logic_and_expr(e) {
            self.parse_expr(expr_node);
          }
        } else if e.op == BinaryOp::LogicalOr {
          if !self.parse_logic_or_expr(e) {
            self.parse_expr(expr_node)
          }
        } else if e.op == BinaryOp::NullishCoalescing {
          if !self.parse_nullish_coalescing_expr(e) {
            self.parse_expr(expr_node);
          }
        } else {
          // 其它类型的写法，比如 someVar || <div>hello</div> 都没有实际意义，假设不会出现这种写法。直接当作表达式处理。
          self.parse_expr(expr_node);
        }
      }

      Expr::Fn(expr) => {
        self.parse_func_function(expr);
      }
      Expr::Arrow(expr) => {
        self.parse_func_arrow(expr);
      }
      Expr::Object(obj) => {
        emit_error(
          obj.span(),
          "不能直接使用 Object 表达式。如果想打印对象，可将其放到模板字符串中。",
        );
      }
      Expr::Array(e) => emit_error(
        e.span(),
        "不能直接使用数组表达式。如果是想渲染多个元素，请使用 For 组件。",
      ),
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
    let host_ident = self.create_host_ident();

    self.push_expression(tpl_render_const_text(
      ast_create_expr_lit_str(text),
      self.context.is_parent_component(),
      host_ident,
    ))
  }
  fn visit_lit(&mut self, n: &Lit) {
    if let Lit::JSXText(t) = n {
      self.visit_jsx_text(t);
    } else {
      let host_ident = self.create_host_ident();

      self.push_expression(tpl_render_const_text(
        Box::new(Expr::Lit(n.clone())),
        self.context.is_parent_component(),
        host_ident,
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

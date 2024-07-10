use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, Spanned};
use swc_core::common::util::take::Take;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use crate::ast::{ast_create_expr_call, ast_create_expr_ident, ast_create_expr_lit_str};
use crate::common::{emit_error, JINGE_IMPORT_CREATE_ELE, JINGE_IMPORT_CREATE_ELE_A};
use crate::parser::attrs::AttrStore;
use crate::parser::tpl::gen_text_render_func;

mod tpl;
mod attrs;


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
  expressions: Box<Vec<Box<Expr>>>,
}

pub struct TemplateParser {
  context: Context,
  stack: Vec<Context>,
}

impl TemplateParser {
  pub fn new() -> Self {
    let root_context = Context {
      parent: Parent::Null,
      expressions: Box::new(vec![]),
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
    let elems: Vec<Option<ExprOrSpread>> = self.context.expressions.take().into_iter()
      .map(|e| {
        Some(ExprOrSpread {
          spread: None,
          expr: e,
        })
      })
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
    let mut attrs  = self.parse_attrs(tn, n);
    let is_svg = tn.as_ref() == "svg";
    self.push_context(Parent::Html(Html { is_svg }));
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let context = self.pop_context();
    let mut args = vec![ExprOrSpread {
      spread: None,
      expr: ast_create_expr_lit_str(tn.as_ref(), Some(tn.span())),
    }];
    let has_attrs = !attrs.lit_props.is_empty();
    if has_attrs {
      let x = Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: attrs.lit_props.take().into_iter().map(|(prop, val)| {
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(prop),
            value: Box::new(Expr::Lit(val))
          })))
        }).collect()
      }));
      args.push(ExprOrSpread {
        spread: None,
        expr: x
      });
    }
    if !context.expressions.is_empty() {
      args.append(
        &mut context
          .expressions
          .into_iter()
          .map(|expr| ExprOrSpread { spread: None, expr })
          .collect::<Vec<ExprOrSpread>>(),
      );
    }
    self.context.expressions.push(ast_create_expr_call(
      ast_create_expr_ident(if has_attrs {
        JINGE_IMPORT_CREATE_ELE_A.1
      } else {
        JINGE_IMPORT_CREATE_ELE.1
      }),
      args,
    ));
  }
  fn parse_component_element(&mut self) {

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
      emit_error(n.opening.name.span(), "未知的 JSX 格式，opening.name 未找到");
      return;
    };
    // let tag = tn.as_ref();
    // println!("visit jsx ele: {}", tag);
    match tn.as_ref().chars().next() {
      Some(c) if c.is_ascii_uppercase() => {}
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
    if !text.is_empty() {
      self.context.expressions.push(ast_create_expr_lit_str(text, Some(n.span())));
    }
  }

  fn visit_lit(&mut self, n: &Lit) {
    if let Parent::Html(_) = &self.context.parent {
      self.context.expressions.push(Box::new(Expr::Lit(n.clone())));
    } else {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.context.expressions.push(gen_text_render_func(Box::new(e)));
    }
  }
}

use crate::ast::{
  ast_create_arg_expr, ast_create_expr_call, ast_create_expr_ident, ast_create_expr_lit_bool,
  ast_create_expr_lit_str, ast_create_expr_lit_string, ast_create_expr_this,
};
use crate::common::{
  emit_error, JINGE_EL_IDENT, JINGE_IMPORT_ADD_EVENT, JINGE_IMPORT_CREATE_ELE,
  JINGE_IMPORT_CREATE_ELE_A, JINGE_IMPORT_TEXT_RENDER_FN,
};
use swc_core::common::util::take::Take;
use swc_core::common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use tpl::{tpl_lit_obj, tpl_push_ele_code, tpl_set_ref_code};

mod attrs;
mod tpl;
mod expr;

#[derive(Debug)]
enum Parent {
  Component,
  Html(Html),
  Null,
}

#[derive(Debug)]
struct Html {
  is_svg: bool,
}

struct Context {
  parent: Parent,
  expressions: Box<Vec<Box<Expr>>>,
}

impl Context {
  pub fn is_parent_svg(&self) -> bool {
    matches!(self.parent, Parent::Html(ref v) if v.is_svg)
  }
  pub fn is_parent_component(&self) -> bool {
    matches!(self.parent, Parent::Component | Parent::Null)
  }
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
    let elems: Vec<Option<ExprOrSpread>> = self
      .context
      .expressions
      .take()
      .into_iter()
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
    let mut attrs = self.parse_attrs(n);
    let is_svg = tn.as_ref() == "svg";
    self.push_context(Parent::Html(Html { is_svg }));
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let children_context = self.pop_context();
    let callee_ident = if self.context.is_parent_svg() || tn.sym.eq("svg") {
      if !attrs.lit_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.1
      } else {
        JINGE_IMPORT_CREATE_ELE.1
      }
    } else {
      if !attrs.lit_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.1
      } else {
        JINGE_IMPORT_CREATE_ELE.1
      }
    };
    let mut args = vec![ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(
      Str::from(tn.sym.clone()),
    ))))];
    let set_ref_code = attrs.ref_mark.take().map(|r| tpl_set_ref_code(r));
    let push_ele_code = if self.context.is_parent_component() {
      Some(tpl_push_ele_code())
    } else {
      None
    };
    if !attrs.lit_props.is_empty() {
      args.push(ast_create_arg_expr(tpl_lit_obj(attrs.lit_props)));
    }
    if !children_context.expressions.is_empty() {
      args.append(
        &mut children_context
          .expressions
          .into_iter()
          .map(|expr| ExprOrSpread { spread: None, expr })
          .collect::<Vec<ExprOrSpread>>(),
      );
    }

    let output = if set_ref_code.is_some() || push_ele_code.is_some() || !attrs.evt_props.is_empty()
    {
      let mut stmts: Vec<Stmt> = vec![Stmt::Decl(Decl::Var(Box::new(VarDecl {
        ctxt: SyntaxContext::empty(),
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
          span: DUMMY_SP,
          name: Pat::Ident(BindingIdent {
            id: Ident::from(JINGE_EL_IDENT),
            type_ann: None,
          }),
          init: Some(ast_create_expr_call(
            ast_create_expr_ident(callee_ident),
            args,
          )),
          definite: false,
        }],
      })))];
      attrs.evt_props.into_iter().for_each(|evt| {
        let mut args = vec![
          ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT)),
          ast_create_arg_expr(ast_create_expr_lit_string(evt.event_name)),
          ast_create_arg_expr(evt.event_handler),
        ];
        if evt.capture {
          args.push(ast_create_arg_expr(ast_create_expr_lit_bool(true)));
        }
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_ADD_EVENT.1), args),
        }))
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
        arg: Some(ast_create_expr_ident(JINGE_EL_IDENT)),
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

    self.context.expressions.push(output);
  }
  fn parse_component_element(&mut self) {}
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
      self
        .context
        .expressions
        .push(ast_create_expr_lit_str(text, Some(n.span())));
    }
  }

  fn visit_lit(&mut self, n: &Lit) {
    if let Parent::Html(_) = &self.context.parent {
      self
        .context
        .expressions
        .push(Box::new(Expr::Lit(n.clone())));
    } else {
      self.context.expressions.push(ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.1),
        vec![
          ast_create_arg_expr(ast_create_expr_this()),
          ast_create_arg_expr(Box::new(Expr::Lit(n.clone()))),
        ],
      ));
    }
  }
}

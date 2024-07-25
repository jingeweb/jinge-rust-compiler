use swc_core::{
  atoms::Atom,
  common::{Spanned, SyntaxContext, DUMMY_SP},
  ecma::{
    ast::*,
    visit::{Visit, VisitAll, VisitAllWith, VisitMut, VisitMutWith},
  },
};

use crate::{
  ast::{
    ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
    ast_create_expr_this,
  },
  common::{
    emit_error, JINGE_IMPORT_DYM_PATH_WATCHER, JINGE_IMPORT_EXPR_WATCHER, JINGE_IMPORT_PATH_WATCHER,
  },
};

enum Root {
  None,
  This,
  Id(Atom),
}

pub struct ExprAttrVisitor {
  meet_error: bool,
  expressions: Vec<Box<Expr>>,
  level: usize,
}
impl ExprAttrVisitor {
  pub fn new() -> Self {
    Self {
      level: 0,
      meet_error: false,
      expressions: vec![],
    }
  }
  fn new_with_level(level: usize) -> Self {
    Self {
      level,
      meet_error: false,
      expressions: vec![],
    }
  }

  pub fn parse(&mut self, expr: &Expr) -> Option<Box<Expr>> {
    self.visit_expr(expr);
    if self.meet_error || self.expressions.is_empty() {
      return None;
    }
    // 如果表达式整个是一个 MemberExpr，则不需要使用 ExprWatcher 进一步封装。
    if matches!(expr, Expr::Member(_)) {
      self.expressions.pop()
    } else {
      let mut expr = expr.clone();
      let mut x = vec![];
      x.append(&mut self.expressions);
      let mut rep = MemberExprReplaceVisitor::new();
      rep.visit_mut_expr(&mut expr);
      let args = vec![
        ast_create_arg_expr(Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: x
            .into_iter()
            .map(|e| Some(ast_create_arg_expr(e)))
            .collect(),
        }))),
        ast_create_arg_expr(ast_create_expr_arrow_fn(
          rep.params,
          Box::new(BlockStmtOrExpr::Expr(Box::new(expr))),
        )),
      ];
      Some(ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_EXPR_WATCHER.1),
        args,
      ))
    }
  }
}
impl VisitAll for ExprAttrVisitor {
  fn visit_expr(&mut self, node: &Expr) {
    if self.meet_error {
      return;
    }
    node.visit_children_with(self);
  }
  fn visit_member_expr(&mut self, node: &MemberExpr) {
    if self.meet_error {
      return;
    }
    let mut mem_parser = MemberExprVisitor::new(self.level);
    mem_parser.visit_member_expr(node);
    if mem_parser.meet_error || mem_parser.path.is_empty() || matches!(&mem_parser.root, Root::None)
    {
      self.meet_error = true;
      return;
    }

    let mut args: Vec<ExprOrSpread> = Vec::with_capacity(mem_parser.path.len() + 2);
    args.push(match mem_parser.root {
      Root::This => ast_create_arg_expr(ast_create_expr_this()),
      Root::Id(id) => ast_create_arg_expr(Box::new(Expr::Ident(Ident::from(id)))),
      Root::None => unreachable!(),
    });
    args.push(ast_create_arg_expr(Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: mem_parser
        .path
        .into_iter()
        .map(|p| Some(ast_create_arg_expr(p)))
        .collect(),
    }))));
    if self.level == 0 {
      args.push(ast_create_arg_expr(Box::new(Expr::Lit(Lit::Bool(
        Bool::from(true),
      )))));
    }
    self.expressions.push(ast_create_expr_call(
      ast_create_expr_ident(if mem_parser.computed {
        JINGE_IMPORT_DYM_PATH_WATCHER.1
      } else {
        JINGE_IMPORT_PATH_WATCHER.1
      }),
      args,
    ))
  }
}

struct MemberExprReplaceVisitor {
  count: usize,
  params: Vec<Pat>,
}
impl MemberExprReplaceVisitor {
  fn new() -> Self {
    Self {
      count: 0,
      params: vec![],
    }
  }
}
impl VisitMut for MemberExprReplaceVisitor {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    match node {
      Expr::Member(_) => {
        let id = Ident::from(format!("a{}", self.count));
        let p = Pat::Ident(BindingIdent {
          id: id.clone(),
          type_ann: None,
        });
        self.params.push(p);
        self.count += 1;
        *node = Expr::Ident(id);
      }
      _ => node.visit_mut_children_with(self),
    }
  }
}

struct MemberExprVisitor {
  root: Root,
  path: Vec<Box<Expr>>,
  meet_error: bool,
  meet_private: bool,
  level: usize,
  computed: bool,
}
impl MemberExprVisitor {
  fn new(level: usize) -> Self {
    Self {
      level,
      root: Root::None,
      path: vec![],
      meet_error: false,
      meet_private: false,
      computed: false,
    }
  }
}
impl Visit for MemberExprVisitor {
  fn visit_member_expr(&mut self, node: &MemberExpr) {
    match node.obj.as_ref() {
      Expr::This(_) => {
        self.root = Root::This;
      }
      Expr::Ident(id) => {
        if !id.sym.starts_with('_') {
          self.root = Root::Id(id.sym.clone());
        } else {
          self.meet_private = true;
        }
      }
      Expr::Member(expr) => {
        self.visit_member_expr(expr);
      }
      _ => {
        emit_error(node.obj.span(), "不支持该类型的表达式");
        self.meet_error = true;
      }
    }
    if self.meet_error || self.meet_private {
      return;
    }
    match &node.prop {
      MemberProp::Ident(id) => {
        if id.sym.starts_with('_') {
          self.meet_private = true;
        } else {
          self
            .path
            .push(Box::new(Expr::Lit(Lit::Str(Str::from(id.sym.clone())))));
        }
      }
      MemberProp::PrivateName(_) => {
        self.meet_private = true;
      }
      MemberProp::Computed(c) => {
        let expr = c.expr.as_ref();
        match expr {
          Expr::Lit(v) => match v {
            Lit::Str(s) => {
              if s.value.starts_with('_') {
                self.meet_private = true;
              } else {
                self.path.push(Box::new(Expr::Lit(v.clone())))
              }
            }
            Lit::Num(_) => self.path.push(Box::new(Expr::Lit(v.clone()))),
            _ => {
              self.meet_error = true;
              emit_error(v.span(), "不支持该常量作为属性");
            }
          },

          _ => {
            if let Some(result) = ExprAttrVisitor::new_with_level(self.level + 1).parse(expr) {
              self.computed = true;
              self.path.push(result);
            }
          }
        }
      }
    }
  }
}

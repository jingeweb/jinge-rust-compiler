use std::rc::Rc;

use hashbrown::HashSet;
use swc_common::SyntaxContext;
use swc_core::{
  atoms::Atom,
  common::{Spanned, DUMMY_SP},
  ecma::{
    ast::*,
    visit::{Visit, VisitMut, VisitMutWith},
  },
};
use swc_ecma_visit::VisitWith;

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
#[derive(Debug)]
pub struct SimpleExprParseResult {
  pub vm: Box<Expr>,
  pub is_this: bool,
  pub path: Box<Expr>,
  pub not_op: i8,
}

#[derive(Debug)]
pub enum ExprParseResult {
  None,
  Simple(SimpleExprParseResult),
  Complex(Box<Expr>),
}

type ExcludeRoots = Option<Rc<HashSet<Atom>>>;

pub struct ExprVisitor {
  no_watch: bool,
  expressions: Vec<Box<Expr>>,
  level: usize,
  simple_result: Option<SimpleExprParseResult>,
  exclude_roots: ExcludeRoots,
}

impl ExprVisitor {
  pub fn new() -> Self {
    Self::new_with_level(0, None)
  }
  pub fn new_with_exclude_roots(exclude_roots: ExcludeRoots) -> Self {
    Self::new_with_level(0, exclude_roots)
  }
  fn new_with_level(level: usize, watch_exclude_roots: ExcludeRoots) -> Self {
    Self {
      level,
      no_watch: false,
      expressions: vec![],
      simple_result: None,
      exclude_roots: watch_exclude_roots,
    }
  }

  pub fn parse(&mut self, expr: &Expr) -> ExprParseResult {
    self.visit_expr(expr);
    if self.no_watch || self.expressions.is_empty() {
      return ExprParseResult::None;
    }
    if self.expressions.len() > 1 {
      // 如果有 >= 2 个 member expr ，则不可能是 simple result
      return ExprParseResult::Complex(self.covert(expr));
    }
    if let Some(mut sr) = self.simple_result.take() {
      // 如果表达式只包含一个 member expr，且整个表达式是：一个 member expr 或 ! + member expr 或 !! + member expr
      // 则作为 Simple Result 返回。也就是对于 {this.submitting} 或 {!this.submitting} 一类的写法简化生成的代码。
      let not_op: i8 = match expr {
        Expr::Member(_) => 0,
        Expr::Unary(e) => match e.op {
          UnaryOp::Bang => match &*e.arg {
            Expr::Unary(e) => match &*e.arg {
              Expr::Member(_) => 2,
              _ => -1,
            },
            Expr::Member(_) => 1,
            _ => -1,
          },
          _ => -1,
        },
        _ => -1,
      };

      if not_op >= 0 {
        sr.not_op = not_op;
        ExprParseResult::Simple(sr)
      } else {
        // println!("{:?}", self.expressions.first().unwrap());
        ExprParseResult::Complex(self.covert(expr))
      }
    } else {
      // 如果 simple_result 为 None，则说明第一个 member expr 有 computed 属性
      let x = self.covert(expr);
      ExprParseResult::Complex(x)
    }
  }
  fn covert(&mut self, expr: &Expr) -> Box<Expr> {
    let mut expr = expr.clone();

    let mut rep = MemberExprReplaceVisitor::new();
    rep.visit_mut_expr(&mut expr);
    // println!("{:?}", expr);
    if matches!(&expr, Expr::Ident(_)) && self.expressions.len() == 1 {
      // 如果转换后的表达式是一个简单的 Ident，并且 expressions 只有一个，说明不需要使用 ExprWatcher 包裹。
      // 比如 state.arr[state.b] 最后会被替换为 ExprWatcher(DymPathWatcher(state, ['arr', PathWatcher(state, 'b')]), (v) => v)
      // 外层的 ExprWatcher 是不需要的，可以直接返回 DymPathWatcher
      return self.expressions.pop().unwrap();
    };

    let mut x = vec![];
    x.append(&mut self.expressions);
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
    ast_create_expr_call(
      ast_create_expr_ident(JINGE_IMPORT_EXPR_WATCHER.local()),
      args,
    )
  }
  fn inner_parse(&mut self, expr: &Expr) -> Option<Box<Expr>> {
    self.visit_expr(expr);
    if self.no_watch || self.expressions.is_empty() {
      return None;
    }
    // 如果表达式整个是一个 MemberExpr，则不需要使用 ExprWatcher 进一步封装。
    if matches!(expr, Expr::Member(_)) {
      self.expressions.pop()
    } else {
      Some(self.covert(expr))
    }
  }
}
impl Visit for ExprVisitor {
  fn visit_expr(&mut self, node: &Expr) {
    if self.no_watch {
      return;
    }
    node.visit_children_with(self);
  }
  fn visit_member_expr(&mut self, node: &MemberExpr) {
    if self.no_watch {
      return;
    }
    let mut mem_parser = MemberExprVisitor::new(self.level, self.exclude_roots.clone());
    mem_parser.visit_member_expr(node);
    if mem_parser.meet_error || mem_parser.path.is_empty() || matches!(&mem_parser.root, Root::None)
    {
      self.no_watch = true;
      return;
    }

    let mut args: Vec<ExprOrSpread> = Vec::with_capacity(mem_parser.path.len() + 2);
    let mut is_this = false;
    let target = match mem_parser.root {
      Root::This => {
        is_this = true;
        ast_create_expr_this()
      }
      Root::Id(id) => Box::new(Expr::Ident(Ident::from(id))),
      Root::None => unreachable!(),
    };
    let watch_path = Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: mem_parser
        .path
        .into_iter()
        .map(|p| Some(ast_create_arg_expr(p)))
        .collect(),
    }));

    if self.level == 0 && !mem_parser.computed && self.expressions.is_empty() {
      // 如果没有 computed 属性，且是第一层的第一个 member expr，则先假设整个表达式都只有这一个 member expr 保存 vm 和 path
      // 待表达式整体全部 visit 结束后，再根据最终的结果看是否使用这个 simple result 作为返回数据。
      self.simple_result = Some(SimpleExprParseResult {
        vm: target.clone(),
        path: watch_path.clone(),
        not_op: 0,
        is_this,
      })
    }

    args.push(ast_create_arg_expr(target));
    args.push(ast_create_arg_expr(watch_path));
    if self.level == 0 {
      args.push(ast_create_arg_expr(Box::new(Expr::Lit(Lit::Bool(
        Bool::from(true),
      )))));
    }
    self.expressions.push(ast_create_expr_call(
      ast_create_expr_ident(if mem_parser.computed {
        JINGE_IMPORT_DYM_PATH_WATCHER.local()
      } else {
        JINGE_IMPORT_PATH_WATCHER.local()
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
  exclude_roots: ExcludeRoots,
}
impl MemberExprVisitor {
  fn new(level: usize, exclude_roots: ExcludeRoots) -> Self {
    Self {
      level,
      root: Root::None,
      path: vec![],
      meet_error: false,
      meet_private: false,
      computed: false,
      exclude_roots,
    }
  }
}
impl Visit for MemberExprVisitor {
  fn visit_member_expr(&mut self, node: &MemberExpr) {
    println!("{:?}", node.obj);
    match node.obj.as_ref() {
      Expr::This(_) => {
        self.root = Root::This;
      }
      Expr::Ident(id) => {
        if let Some(exclude_roots) = &self.exclude_roots {
          if exclude_roots.contains(&id.sym) {
            self.meet_private = true;
          } else {
            self.root = Root::Id(id.sym.clone());
          }
        } else {
          self.root = Root::Id(id.sym.clone());
        }
      }
      Expr::Member(expr) => {
        self.visit_member_expr(expr);
      }
      Expr::OptChain(expr) => {
        emit_error(node.obj.span(), "不支持 OptChain 表达式");
        self.meet_error = true;
      }
      Expr::Call(call) => {
        // TODO: 支持形如 `state.a().b.c` 这样的，函数调用的结果作为 ViewModel 进一步取其上的属性。
        // 这种情况会比较复杂，因为 watch 的目标是动态的，需要用 PathWatcher/DymPathWatcher/ExprWatcher 之外更复杂的 watcher 机制来支持。
        // 后续如果这种情况是刚需再考虑支持。
        emit_error(
          call.span(),
          "暂不支持该 Call 表达式作为 Member Expr 的 object",
        );
        self.meet_error = true;
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
        self
          .path
          .push(Box::new(Expr::Lit(Lit::Str(Str::from(id.sym.clone())))));
      }
      MemberProp::PrivateName(_) => {
        self.meet_private = true;
      }
      MemberProp::Computed(c) => {
        let expr = c.expr.as_ref();
        match expr {
          Expr::Lit(v) => match v {
            Lit::Str(_) | Lit::Num(_) => self.path.push(Box::new(Expr::Lit(v.clone()))),
            _ => {
              self.meet_error = true;
              emit_error(v.span(), "不支持该常量作为属性");
            }
          },

          _ => {
            if let Some(result) =
              ExprVisitor::new_with_level(self.level + 1, self.exclude_roots.clone())
                .inner_parse(expr)
            {
              self.computed = true;
              self.path.push(result);
            } else {
              todo!("xxx")
            }
          }
        }
      }
    }
  }
}

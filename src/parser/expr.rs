use swc_core::{
  atoms::Atom,
  common::Spanned,
  ecma::{ast::*, visit::Visit},
};

use crate::common::emit_error;

pub struct AttrWatchExpr {}

pub enum AttrExpr {
  Pure,
  Watch(AttrWatchExpr),
}

pub fn parse_expr_attr(val: &Expr) -> AttrExpr {
  let mut parser = ExprAttrVisitor::new();
  parser.visit_expr(val);
  if parser.computed_paths.is_empty() && parser.const_paths.is_empty() {
    return AttrExpr::Pure;
  }
  if parser.computed_paths.is_empty() {
    let x = 
  }

  AttrExpr::Pure
}

pub struct WatchExpr {
  pub root: Root,
  pub path: Vec<PathItem>,
}
impl From<ExprAttrWalker> for WatchExpr {
  fn from(value: ExprAttrWalker) -> Self {
    Self {
      root: value.root,
      path: value.path,
    }
  }
}
struct ExprAttrVisitor {
  const_paths: Vec<WatchExpr>,
  computed_paths: Vec<WatchExpr>,
}

impl ExprAttrVisitor {
  pub fn new() -> Self {
    Self {
      const_paths: vec![],
      computed_paths: vec![],
    }
  }
}

struct ExprAttrWalker {
  root: Root,
  computed: ComputedType,
  path: Vec<PathItem>,
  meet_private: bool,
}
impl ExprAttrWalker {
  fn new() -> Self {
    Self {
      root: Root::None,
      computed: ComputedType::None,
      path: vec![],
      meet_private: false,
    }
  }

  fn walk(&mut self, n: &MemberExpr) {
    match n.obj.as_ref() {
      Expr::This(_) => {
        self.root = Root::This;
      }
      Expr::Ident(e) => {
        if !e.sym.starts_with('_') {
          self.root = Root::Id(e.sym.clone());
        } else {
          // 如果 ident 是下划线打头，则认定为不进行 watch 监控。
          self.meet_private = true
        }
      }
      Expr::Member(e) => {
        self.walk(e);
      }
      _ => {
        emit_error(n.obj.span(), "不支持该类型的表达式");
        self.root = Root::None;
        self.meet_private = true;
      }
    }
    if self.meet_private || matches!(self.root, Root::None) {
      return;
    }
    match &n.prop {
      MemberProp::Computed(c) => match c.expr.as_ref() {
        Expr::Lit(v) => match v {
          Lit::Str(s) => {
            let s = &s.value;
            if !s.starts_with('_') {
              if matches!(self.computed, ComputedType::None) {
                self.computed = ComputedType::Const;
              }
            } else {
              // 如果 property 是下划线打头，则认定为不进行 watch 监控。
              self.meet_private = true;
            }
            self.path.push(PathItem::Const(s.clone()));
          }
          Lit::Num(n) => {
            if matches!(self.computed, ComputedType::None) {
              self.computed = ComputedType::Const;
            }
            self
              .path
              .push(PathItem::Const(Atom::from(n.value.to_string())));
          }
          _ => {
            emit_error(v.span(), "不支持该常量作为属性");
          }
        },
        _ => {
          self.computed = ComputedType::Expr;
          self.path.push(PathItem::Computed(c.expr.clone()));
        }
      },
      MemberProp::Ident(c) => {
        if !c.sym.starts_with('_') {
          if matches!(self.computed, ComputedType::None) {
            self.computed = ComputedType::Const;
          }
        } else {
          // 下划线打头的 property 不进行 watch。path 中只要有一个 item 是 public 的，就需要进行 watch
          self.meet_private = true
        }
        self.path.push(PathItem::Const(c.sym.clone()));
      }
      MemberProp::PrivateName(c) => {
        self.path.push(PathItem::PrivateName(c.name.clone()));
      }
    };
  }
}
enum PathItem {
  PrivateName(Atom),
  Const(Atom),
  Computed(Box<Expr>),
}
enum ComputedType {
  None,
  Const,
  Expr,
}
enum Root {
  None,
  This,
  Id(Atom),
}

impl Visit for ExprAttrVisitor {
  fn visit_member_expr(&mut self, node: &MemberExpr) {
    let mut walker = ExprAttrWalker::new();
    walker.walk(node);
    if matches!(walker.root, Root::None) {
      return;
    }
    // if matches!(walker.computed, ComputedType::None) {
    //   return;
    // }
    // self.is_const = false;
    match walker.computed {
      ComputedType::None => (),
      ComputedType::Const => self.const_paths.push(WatchExpr::from(walker)),
      ComputedType::Expr => self.computed_paths.push(WatchExpr::from(walker)),
    }
  }
}

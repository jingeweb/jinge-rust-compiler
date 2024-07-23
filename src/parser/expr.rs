use swc_core::{
  atoms::Atom,
  common::Spanned,
  ecma::{ast::*, visit::Visit},
};

use crate::common::emit_error;

pub struct AttrExpr {
  pub name: Atom,
  pub is_const: bool,
}

pub fn parse_expr_attr(name: Atom, val: &Expr) -> AttrExpr {
  let mut parser = ExprAttrVisitor::new();
  parser.visit_expr(val);

  AttrExpr {
    name,
    is_const: parser.is_const,
  }
}

struct ExprAttrVisitor<'a> {
  is_const: bool,
  watch_paths: Vec<PathItem<'a>>,
  computed_member_exprs: Vec<&'a Expr>,
}

impl<'a> ExprAttrVisitor<'a> {
  pub fn new() -> Self {
    Self {
      is_const: false,
      watch_paths: vec![],
      computed_member_exprs: vec![],
    }
  }
}

struct ExprAttrWalker<'a> {
  root: Root,
  computed: ComputedType,
  path: Vec<PathItem<'a>>,
  path_need_watch: bool,
}
impl<'a> ExprAttrWalker<'a> {
  fn new() -> Self {
    Self {
      root: Root::None,
      computed: ComputedType::None,
      path: vec![],
      path_need_watch: false,
    }
  }
  // #[inline]
  // fn prepend_path(&mut self, p: PathItem<'a>) {
  //   let mut new_paths = Vec::with_capacity(self.paths.len() + 1);
  //   new_paths.push(p);
  //   new_paths.append(&mut self.paths);
  //   self.paths = new_paths;
  // }
  fn walk(&mut self, n: &'a MemberExpr) {
    match n.obj.as_ref() {
      Expr::This(_) => {
        self.root = Root::This;
      }
      Expr::Ident(e) => {
        if !e.sym.starts_with('_') {
          self.root = Root::Id(e.sym.clone());
        } else {
          // 如果 ident 是下划线打头，则认定为不进行 watch 监控。
        }
      }
      Expr::Member(e) => {
        self.walk(e);
      }
      _ => {
        emit_error(n.obj.span(), "不支持该类型的表达式");
      }
    }
    if matches!(self.root, Root::None) {
      return;
    }
    let prop = &n.prop;
    match prop {
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
          self.path.push(PathItem::Computed(c.expr.as_ref()));
        }
      },
      MemberProp::Ident(c) => {
        if !c.sym.starts_with('_') {
          if matches!(self.computed, ComputedType::None) {
            self.computed = ComputedType::Const;
          }
        } else {
          // 下划线打头的 property 不进行 watch。path 中只要有一个 item 是 public 的，就需要进行 watch
        }
        self.path.push(PathItem::Const(c.sym.clone()));
      }
      MemberProp::PrivateName(c) => {
        self.path.push(PathItem::PrivateName(c.name.clone()));
      }
    };
  }
}
enum PathItem<'a> {
  PrivateName(Atom),
  Const(Atom),
  Computed(&'a Expr),
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

impl Visit for ExprAttrVisitor<'_> {
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
      ComputedType::None => return,
      ComputedType::Const => self.watch_paths.push(walker.path),
      ComputedType::Expr => {}
    }
  }
}

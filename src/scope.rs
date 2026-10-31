//! Per-transform lexical symbols. SWC binding Ids remain authoritative across
//! nested functions, blocks, destructuring, and template callback rewrites.
use crate::common::emit_warn;
use hashbrown::HashMap;
use std::cell::RefCell;
use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Clone, Copy, Default, PartialEq)]
enum Source {
  Reactive,
  Plain,
  #[default]
  Unknown,
}
#[derive(Clone, Copy)]
struct Symbol {
  source: Source,
  constant: bool,
}
#[derive(Default)]
struct Symbols {
  bindings: HashMap<Id, Symbol>,
  // Frames describe lexical declaration ownership; lookups use resolver Ids,
  // so bindings remain usable when templates are parsed out of visit order.
  scopes: Vec<Vec<Id>>,
  vm_imports: Vec<Id>,
}
thread_local! { static SYMBOLS: RefCell<Symbols> = RefCell::default(); }
pub struct ScopeGuard(Option<Symbols>);
impl ScopeGuard {
  pub fn enter(program: &Program) -> Self {
    let mut symbols = Symbols::default();
    symbols.scopes.push(vec![]);
    program.visit_with(&mut symbols);
    Self(Some(SYMBOLS.with(|s| s.replace(symbols))))
  }
}
impl Drop for ScopeGuard {
  fn drop(&mut self) {
    SYMBOLS.with(|s| {
      s.replace(self.0.take().unwrap());
    });
  }
}
impl Symbols {
  fn source(&self, expr: &Expr) -> Source {
    match expr {
      Expr::Call(call) if matches!(&call.callee, Callee::Expr(c) if matches!(&**c, Expr::Ident(id) if self.vm_imports.contains(&id.to_id()))) => {
        Source::Reactive
      }
      Expr::Object(_) | Expr::Array(_) | Expr::Lit(_) => Source::Plain,
      Expr::Ident(id) => self
        .bindings
        .get(&id.to_id())
        .map(|s| s.source)
        .unwrap_or_default(),
      Expr::Paren(p) => self.source(&p.expr),
      // A member of a vm can also be a primitive; it is still a valid source
      // path for template watches. Unknown members stay unknown.
      Expr::Member(m) if self.source(&m.obj) == Source::Reactive => Source::Reactive,
      _ => Source::Unknown,
    }
  }
  fn frame<T: VisitWith<Self>>(&mut self, node: &T) {
    self.scopes.push(vec![]);
    node.visit_children_with(self);
    self.scopes.pop();
  }
}
impl Visit for Symbols {
  fn visit_import_decl(&mut self, node: &ImportDecl) {
    if node.src.value == "jinge" {
      for spec in &node.specifiers {
        if let ImportSpecifier::Named(n) = spec {
          let vm = match &n.imported {
            Some(ModuleExportName::Ident(id)) => id.sym == "vm",
            Some(ModuleExportName::Str(s)) => s.value == "vm",
            None => n.local.sym == "vm",
          };
          if vm && !self.vm_imports.contains(&n.local.to_id()) {
            self.vm_imports.push(n.local.to_id());
          }
        }
      }
    }
  }
  fn visit_module(&mut self, node: &Module) {
    // Imports are hoisted regardless of their position in the file.
    for item in &node.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(i)) = item {
        self.visit_import_decl(i);
      }
    }
    node.visit_children_with(self);
  }
  fn visit_function(&mut self, node: &Function) {
    self.frame(node);
  }
  fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
    self.frame(node);
  }
  fn visit_block_stmt(&mut self, node: &BlockStmt) {
    self.frame(node);
  }
  fn visit_catch_clause(&mut self, node: &CatchClause) {
    self.frame(node);
  }
  fn visit_binding_ident(&mut self, id: &BindingIdent) {
    // Parameters and catch bindings have unknown provenance and are mutable.
    self.bindings.entry(id.id.to_id()).or_insert(Symbol {
      source: Source::Unknown,
      constant: false,
    });
  }
  fn visit_var_decl(&mut self, node: &VarDecl) {
    for decl in &node.decls {
      if let Pat::Ident(id) = &decl.name {
        let id = id.id.to_id();
        let source = decl
          .init
          .as_ref()
          .map(|e| self.source(e))
          .unwrap_or_default();
        self.bindings.insert(
          id.clone(),
          Symbol {
            source,
            constant: node.kind == VarDeclKind::Const,
          },
        );
        self.scopes.last_mut().unwrap().push(id);
      } else {
        let mut bindings = PatternBindings::default();
        decl.name.visit_with(&mut bindings);
        for id in bindings.0 {
          self.bindings.insert(
            id.clone(),
            Symbol {
              source: Source::Unknown,
              constant: node.kind == VarDeclKind::Const,
            },
          );
          self.scopes.last_mut().unwrap().push(id);
        }
      }
      decl.visit_children_with(self);
    }
  }
  fn visit_assign_expr(&mut self, node: &AssignExpr) {
    if let AssignTarget::Simple(SimpleAssignTarget::Ident(id)) = &node.left
      && let Some(symbol) = self.bindings.get_mut(&id.id.to_id())
    {
      symbol.source = Source::Unknown;
    }
    node.visit_children_with(self);
  }
}
pub fn warn_member(node: &MemberExpr) {
  let mut object = node.obj.as_ref();
  loop {
    match object {
      Expr::Member(member) => object = &member.obj,
      Expr::OptChain(chain) => {
        if let OptChainBase::Member(member) = &*chain.base {
          object = &member.obj;
        } else {
          return;
        }
      }
      _ => break,
    }
  }
  if let Expr::Ident(id) = object {
    SYMBOLS.with(|s| {
      if s
        .borrow()
        .bindings
        .get(&id.to_id())
        .is_some_and(|s| s.source == Source::Plain)
      {
        emit_warn(
          node.span(),
          &format!(
            "{} 未使用 vm 包装，此处不会响应数据变化，可能遗漏了 vm 包装",
            id.sym
          ),
        );
      }
    });
  }
}
pub fn warn_mutable_key(id: &Ident) {
  SYMBOLS.with(|s| {
    if s
      .borrow()
      .bindings
      .get(&id.to_id())
      .is_some_and(|s| !s.constant)
    {
      emit_warn(
        id.span,
        "computed key 不是 const 绑定，修改 key 本身不会触发响应式更新",
      );
    }
  });
}

#[derive(Default)]
struct PatternBindings(Vec<Id>);
impl Visit for PatternBindings {
  fn visit_binding_ident(&mut self, id: &BindingIdent) {
    self.0.push(id.id.to_id());
  }
  // Defaults may contain unrelated nested declarations.
  fn visit_expr(&mut self, _: &Expr) {}
}

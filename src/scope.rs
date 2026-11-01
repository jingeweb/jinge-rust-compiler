//! Per-transform lexical symbols. SWC binding Ids remain authoritative across
//! nested functions, blocks, destructuring, and template callback rewrites.
use crate::common::emit_warn;
use hashbrown::{HashMap, HashSet};
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
  snapshot: bool,
}
#[derive(Default)]
struct Symbols {
  bindings: HashMap<Id, Symbol>,
  // Frames describe lexical declaration ownership; lookups use resolver Ids,
  // so bindings remain usable when templates are parsed out of visit order.
  scopes: Vec<Vec<Id>>,
  vm_imports: Vec<Id>,
  map_parameters: HashSet<Id>,
  written: HashSet<Id>,
}
thread_local! { static SYMBOLS: RefCell<Symbols> = RefCell::default(); }
pub struct ScopeGuard(Option<Symbols>);
impl ScopeGuard {
  pub fn enter(program: &Program) -> Self {
    let mut writes = WrittenBindings::default();
    program.visit_with(&mut writes);
    let mut symbols = Symbols {
      written: writes.0,
      ..Default::default()
    };
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
  fn snapshot(&self, expr: &Expr) -> bool {
    // Explicit vm construction defines a new reactive source.
    if matches!(expr, Expr::Call(call) if matches!(&call.callee, Callee::Expr(c) if matches!(&**c, Expr::Ident(id) if self.vm_imports.contains(&id.to_id()))))
    {
      return false;
    }
    let mut reads = SnapshotRead {
      symbols: self,
      found: false,
    };
    expr.visit_with(&mut reads);
    reads.found
  }
  fn template_parameter(&mut self, param: Option<&Pat>, result: Option<&Expr>) {
    let (Some(Pat::Ident(param)), Some(result)) = (param, result) else {
      return;
    };
    let name = swc_core::atoms::Wtf8Atom::new(param.sym.as_str());
    if crate::helper::should_render_as_jsx(result, &Some(name)) {
      self.bindings.entry(param.id.to_id()).or_insert(Symbol {
        source: Source::Reactive,
        constant: false,
        snapshot: false,
      });
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
    self.template_parameter(
      node.params.first().map(|p| &p.pat),
      node
        .body
        .as_ref()
        .and_then(|body| final_return(&body.stmts)),
    );
    self.frame(node);
  }
  fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
    let result = match &*node.body {
      ArrowFunctionBody::Expr(expr) => Some(&**expr),
      ArrowFunctionBody::FunctionBody(body) => final_return(&body.stmts),
    };
    self.template_parameter(node.params.first(), result);
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
      snapshot: false,
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
            snapshot: decl.init.as_ref().is_some_and(|e| self.snapshot(e)),
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
              snapshot: decl
                .init
                .as_ref()
                .is_some_and(|e| self.source(e) == Source::Reactive || self.snapshot(e)),
            },
          );
          self.scopes.last_mut().unwrap().push(id);
        }
      }
      decl.visit_children_with(self);
    }
  }
  fn visit_call_expr(&mut self, node: &CallExpr) {
    // Template map callbacks become For slots. Their data/index parameters are
    // reactive paths even when the collection's provenance is unknown.
    let is_map = match &node.callee {
      Callee::Expr(expr) => match &**expr {
        Expr::Member(member) => {
          matches!(&member.prop, MemberProp::Ident(name) if name.sym == "map")
        }
        Expr::OptChain(chain) => {
          matches!(&*chain.base, OptChainBase::Member(member) if matches!(&member.prop, MemberProp::Ident(name) if name.sym == "map"))
        }
        _ => false,
      },
      _ => false,
    };
    if is_map
      && node.args.len() == 1
      && node.args[0].spread.is_none()
      && let Expr::Arrow(callback) = &*node.args[0].expr
    {
      for param in callback.params.iter().take(2) {
        if let Pat::Ident(param) = param {
          let id = param.id.to_id();
          self.map_parameters.insert(id.clone());
          self.bindings.insert(
            id,
            Symbol {
              source: Source::Reactive,
              constant: false,
              snapshot: false,
            },
          );
        }
      }
    }
    node.visit_children_with(self);
  }
  fn visit_assign_expr(&mut self, node: &AssignExpr) {
    let snapshot = self.snapshot(&node.right);
    if let AssignTarget::Simple(SimpleAssignTarget::Ident(id)) = &node.left
      && let Some(symbol) = self.bindings.get_mut(&id.id.to_id())
    {
      symbol.source = Source::Unknown;
      symbol.snapshot |= snapshot;
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

// Track eager reads only: function bodies execute later and are not snapshots
// merely because their closures mention a reactive source.
struct SnapshotRead<'a> {
  symbols: &'a Symbols,
  found: bool,
}
impl Visit for SnapshotRead<'_> {
  fn visit_expr(&mut self, expr: &Expr) {
    match expr {
      Expr::Ident(id) => self.visit_ident(id),
      Expr::Member(member) if self.symbols.source(&member.obj) == Source::Reactive => {
        self.found = true
      }
      Expr::Arrow(_) | Expr::Fn(_) | Expr::JSXElement(_) | Expr::JSXFragment(_) => {}
      _ => expr.visit_children_with(self),
    }
  }
  fn visit_function(&mut self, _: &Function) {}
  fn visit_ts_type(&mut self, _: &TsType) {}
  fn visit_ident(&mut self, id: &Ident) {
    if self.symbols.map_parameters.contains(&id.to_id())
      || self.symbols.written.contains(&id.to_id())
      || self
        .symbols
        .bindings
        .get(&id.to_id())
        .is_some_and(|s| s.snapshot)
    {
      self.found = true;
    }
  }
}

pub fn warn_snapshot_refs(expr: &Expr) {
  struct SnapshotRefs;
  impl Visit for SnapshotRefs {
    fn visit_expr(&mut self, expr: &Expr) {
      match expr {
        Expr::Ident(id) => self.visit_ident(id),
        Expr::Arrow(_) | Expr::Fn(_) | Expr::JSXElement(_) | Expr::JSXFragment(_) => {}
        _ => expr.visit_children_with(self),
      }
    }
    fn visit_function(&mut self, _: &Function) {}
    fn visit_ident(&mut self, id: &Ident) {
      let (snapshot, written) = SYMBOLS.with(|s| {
        let symbols = s.borrow();
        let binding = id.to_id();
        (
          symbols.bindings.get(&binding).is_some_and(|s| s.snapshot),
          symbols.bindings.contains_key(&binding) && symbols.written.contains(&binding),
        )
      });
      if written {
        emit_warn(
          id.span,
          &format!(
            "局部变量 {} 在当前文件中存在重新赋值，重新赋值不会自动更新模板或重新绑定监听，可能失去响应；请用 vm 成员保存和修改状态，或在 JSX 中完整书写原始响应式路径，避免重新绑定变量",
            id.sym
          ),
        );
      } else if snapshot {
        emit_warn(
          id.span,
          &format!(
            "局部变量 {} 保存了响应式或可变数据的读取结果，此处会失去对原始路径的响应式追踪；请在 JSX 中完整书写原始表达式（如 item.name、state.name 或 props.name），不要先提取为局部变量",
            id.sym
          ),
        );
      }
    }
  }
  expr.visit_with(&mut SnapshotRefs);
}

fn final_return(stmts: &[Stmt]) -> Option<&Expr> {
  match stmts
    .iter()
    .rev()
    .find(|stmt| !matches!(stmt, Stmt::Empty(_)))
  {
    Some(Stmt::Return(stmt)) => stmt.arg.as_deref(),
    _ => None,
  }
}

// Collect writes before provenance analysis so callbacks and declarations may
// appear in either order. Mutating obj.member never counts as rebinding obj.
#[derive(Default)]
struct WrittenBindings(HashSet<Id>);
impl WrittenBindings {
  fn target<T: VisitWith<PatternBindings>>(&mut self, target: &T) {
    let mut bindings = PatternBindings::default();
    target.visit_with(&mut bindings);
    self.0.extend(bindings.0);
  }
}
impl Visit for WrittenBindings {
  fn visit_assign_expr(&mut self, node: &AssignExpr) {
    self.target(&node.left);
    node.visit_children_with(self);
  }
  fn visit_update_expr(&mut self, node: &UpdateExpr) {
    if let Expr::Ident(id) = &*node.arg {
      self.0.insert(id.to_id());
    }
    node.visit_children_with(self);
  }
  fn visit_for_of_stmt(&mut self, node: &ForOfStmt) {
    if let ForHead::Pat(pattern) = &node.left {
      self.target(pattern);
    }
    node.visit_children_with(self);
  }
  fn visit_for_in_stmt(&mut self, node: &ForInStmt) {
    if let ForHead::Pat(pattern) = &node.left {
      self.target(pattern);
    }
    node.visit_children_with(self);
  }
}

//! Per-transform lexical symbols. SWC binding Ids remain authoritative across
//! nested functions, blocks, destructuring, and template callback rewrites.
use crate::common::emit_warn;
use hashbrown::{HashMap, HashSet};
use std::{cell::RefCell, collections::VecDeque};
use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
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
struct Initializer {
  bindings: Vec<Id>,
  // None denotes a deferred callable: do not clone its potentially large body.
  expr: Option<Box<Expr>>,
  destructured: bool,
}

#[derive(Default)]
struct Symbols {
  bindings: HashMap<Id, Symbol>,
  initializers: Vec<Initializer>,
  template_props: HashSet<Id>,
  invalid_component_aliases: HashSet<Id>,
  // Frames describe lexical declaration ownership; lookups use resolver Ids,
  // so bindings remain usable when templates are parsed out of visit order.
  scopes: Vec<Vec<Id>>,
  vm_imports: Vec<Id>,
  intl_imports: HashSet<Id>,
  map_parameters: HashSet<Id>,
  written: HashSet<Id>,
  unresolved_mark: Option<swc_common::Mark>,
}
thread_local! { static SYMBOLS: RefCell<Symbols> = RefCell::default(); }
pub struct ScopeGuard(Option<Symbols>);
impl ScopeGuard {
  pub fn enter(program: &Program, templates: bool, unresolved_mark: swc_common::Mark) -> Self {
    let mut symbols = Symbols {
      unresolved_mark: Some(unresolved_mark),
      ..Default::default()
    };
    // Make hoisted import identities available even while classifying template
    // function parameters (a component may return only an imported t call).
    if let Program::Module(module) = program {
      for item in &module.body {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
          symbols.visit_import_decl(import);
        }
      }
    }
    let guard = Self(Some(SYMBOLS.with(|s| {
      s.replace(Symbols {
        intl_imports: symbols.intl_imports.clone(),
        ..Default::default()
      })
    })));
    if templates {
      let mut writes = WrittenBindings::default();
      program.visit_with(&mut writes);
      symbols.written = writes.0;
      let mut roles = TemplateRoles::default();
      // Collect uses before definitions so forward references and aliases of
      // imported bootstrap are resolved by binding identity, not spelling.
      if let Program::Module(module) = program {
        for item in &module.body {
          if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            roles.visit_import_decl(import);
          }
        }
      }
      program.visit_with(&mut roles);
      for binding_root in &roles.components {
        let mut binding = binding_root;
        let mut seen = HashSet::new();
        while seen.insert(binding.clone()) {
          if symbols.written.contains(binding) {
            symbols.invalid_component_aliases.insert(binding.clone());
            symbols
              .invalid_component_aliases
              .insert(binding_root.clone());
            break;
          }
          if let Some(param) = roles.functions.get(binding) {
            roles.props.insert(param.clone());
            break;
          }
          let Some(alias) = roles.aliases.get(binding) else {
            break;
          };
          binding = alias;
        }
      }
      symbols.template_props = roles.props;
      symbols.scopes.push(vec![]);
      program.visit_with(&mut symbols);
      symbols.resolve_provenance();
    }
    SYMBOLS.with(|s| s.replace(symbols));
    guard
  }
}
impl Drop for ScopeGuard {
  fn drop(&mut self) {
    SYMBOLS.with(|s| {
      s.replace(self.0.take().unwrap());
    });
  }
}
// Only known template functions give their first parameter the props role.
// Returning `node.children` alone is not evidence that a data callback renders.
#[derive(Default)]
struct TemplateRoles {
  functions: HashMap<Id, Id>,
  aliases: HashMap<Id, Id>,
  components: HashSet<Id>,
  props: HashSet<Id>,
  bootstrap_imports: HashSet<Id>,
}
impl TemplateRoles {
  fn parameter(&mut self, name: Option<&Ident>, param: Option<&Pat>, result: Option<&Expr>) {
    let Some(Pat::Ident(param)) = param else {
      return;
    };
    if let Some(name) = name {
      self.functions.insert(name.to_id(), param.id.to_id());
    }
    if result.is_some_and(|expr| crate::helper::should_render_as_jsx(expr, &None)) {
      self.props.insert(param.id.to_id());
    }
  }
  fn callable(&mut self, name: Option<&Ident>, expr: &Expr, component: bool) {
    match expr {
      Expr::Arrow(arrow) => {
        self.parameter(name, arrow.params.first(), None);
        if component && let Some(Pat::Ident(param)) = arrow.params.first() {
          self.props.insert(param.id.to_id());
        }
      }
      Expr::Fn(function) => {
        self.parameter(name, function.function.params.first().map(|p| &p.pat), None);
        if component
          && let Some(Param {
            pat: Pat::Ident(param),
            ..
          }) = function.function.params.first()
        {
          self.props.insert(param.id.to_id());
        }
      }
      Expr::Ident(id) => {
        if component {
          self.components.insert(id.to_id());
        } else if let Some(name) = name {
          self.aliases.insert(name.to_id(), id.to_id());
        }
      }
      Expr::Paren(paren) => self.callable(name, &paren.expr, component),
      _ => (),
    }
  }
}
impl Visit for TemplateRoles {
  fn visit_fn_decl(&mut self, node: &FnDecl) {
    self.parameter(
      Some(&node.ident),
      node.function.params.first().map(|p| &p.pat),
      None,
    );
    node.visit_children_with(self);
  }
  fn visit_var_declarator(&mut self, node: &VarDeclarator) {
    if let (Pat::Ident(name), Some(init)) = (&node.name, &node.init) {
      self.callable(Some(&name.id), init, false);
    }
    node.visit_children_with(self);
  }
  fn visit_function(&mut self, node: &Function) {
    self.parameter(
      None,
      node.params.first().map(|p| &p.pat),
      node.body.as_ref().and_then(|b| final_return(&b.stmts)),
    );
    node.visit_children_with(self);
  }
  fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
    let result = match &*node.body {
      ArrowFunctionBody::Expr(expr) => Some(&**expr),
      ArrowFunctionBody::FunctionBody(body) => final_return(&body.stmts),
    };
    self.parameter(None, node.params.first(), result);
    node.visit_children_with(self);
  }
  fn visit_jsx_opening_element(&mut self, node: &JSXOpeningElement) {
    if let JSXElementName::Ident(id) = &node.name {
      self.components.insert(id.to_id());
    }
    node.visit_children_with(self);
  }
  fn visit_export_decl(&mut self, node: &ExportDecl) {
    // Public PascalCase functions may be consumed as components in another file.
    match &node.decl {
      Decl::Fn(function)
        if function
          .ident
          .sym
          .chars()
          .next()
          .is_some_and(char::is_uppercase) =>
      {
        self.components.insert(function.ident.to_id());
      }
      Decl::Var(var) => {
        for decl in &var.decls {
          if let Pat::Ident(name) = &decl.name
            && name.sym.chars().next().is_some_and(char::is_uppercase)
          {
            self.components.insert(name.id.to_id());
          }
        }
      }
      _ => (),
    }
    node.visit_children_with(self);
  }
  fn visit_named_export(&mut self, node: &NamedExport) {
    if node.src.is_none() && !node.type_only {
      for specifier in &node.specifiers {
        if let ExportSpecifier::Named(specifier) = specifier
          && !specifier.is_type_only
          && let ModuleExportName::Ident(local) = &specifier.orig
        {
          let exported = specifier.exported.as_ref().unwrap_or(&specifier.orig);
          let uppercase = match exported {
            ModuleExportName::Ident(id) => {
              id.sym == "default" || id.sym.chars().next().is_some_and(char::is_uppercase)
            }
            ModuleExportName::Str(_) => false,
          };
          if uppercase {
            self.components.insert(local.to_id());
          }
        }
      }
    }
    node.visit_children_with(self);
  }
  fn visit_export_default_decl(&mut self, node: &ExportDefaultDecl) {
    if let DefaultDecl::Fn(function) = &node.decl
      && let Some(Param {
        pat: Pat::Ident(param),
        ..
      }) = function.function.params.first()
    {
      self.props.insert(param.id.to_id());
    }
    node.visit_children_with(self);
  }
  fn visit_export_default_expr(&mut self, node: &ExportDefaultExpr) {
    self.callable(None, &node.expr, true);
    node.visit_children_with(self);
  }
  fn visit_import_decl(&mut self, node: &ImportDecl) {
    if node.src.value == "jinge" && !node.type_only {
      for spec in &node.specifiers {
        if let ImportSpecifier::Named(named) = spec {
          let imported = match &named.imported {
            Some(ModuleExportName::Ident(id)) => id.sym.as_ref(),
            _ => named.local.sym.as_ref(),
          };
          if imported == "bootstrap" && !named.is_type_only {
            self.bootstrap_imports.insert(named.local.to_id());
          }
        }
      }
    }
  }
  fn visit_call_expr(&mut self, node: &CallExpr) {
    if matches!(&node.callee, Callee::Expr(c) if matches!(&**c, Expr::Ident(id) if self.bootstrap_imports.contains(&id.to_id())))
      && let Some(arg) = node.args.first()
    {
      self.callable(None, &arg.expr, true);
    }
    node.visit_children_with(self);
  }
}

pub fn is_template_props(id: &Ident) -> bool {
  SYMBOLS.with(|symbols| symbols.borrow().template_props.contains(&id.to_id()))
}

impl Symbols {
  fn resolve_provenance(&mut self) {
    let initializers = std::mem::take(&mut self.initializers);
    let mut counts: HashMap<Id, usize> = HashMap::new();
    let mut unstable = self.written.clone();
    for init in &initializers {
      for id in &init.bindings {
        *counts.entry(id.clone()).or_default() += 1;
        // A parameter reinitialized by a var declaration is no longer a
        // reliable props/map source. Do not infer it from one execution path.
        if self.bindings[id].source == Source::Reactive {
          unstable.insert(id.clone());
        }
      }
    }
    unstable.extend(
      counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id)),
    );
    for id in &unstable {
      if let Some(symbol) = self.bindings.get_mut(id) {
        symbol.source = Source::Unknown;
      }
    }

    let mut dependents: HashMap<Id, Vec<usize>> = HashMap::new();
    for (index, init) in initializers.iter().enumerate() {
      let mut references = EagerBindings::default();
      if let Some(expr) = &init.expr {
        expr.visit_with(&mut references);
      }
      for id in references.0 {
        dependents.entry(id).or_default().push(index);
      }
    }
    let mut queue: VecDeque<_> = (0..initializers.len()).collect();
    let mut queued = vec![true; initializers.len()];
    while let Some(index) = queue.pop_front() {
      queued[index] = false;
      let init = &initializers[index];
      let (source, snapshot) = if let Some(expr) = &init.expr {
        let source = self.source(expr);
        (
          source,
          self.snapshot(expr) || (init.destructured && source == Source::Reactive),
        )
      } else {
        (Source::Unknown, false)
      };
      for id in &init.bindings {
        let next_source = if init.destructured || unstable.contains(id) {
          Source::Unknown
        } else {
          source
        };
        let symbol = self.bindings.get_mut(id).unwrap();
        if symbol.source == next_source && (symbol.snapshot || !snapshot) {
          continue;
        }
        // Sources move only from Unknown to Plain/Reactive; snapshot is a
        // monotone bit. Cyclic aliases therefore terminate without guessing.
        debug_assert!(symbol.source == Source::Unknown || symbol.source == next_source);
        symbol.source = next_source;
        symbol.snapshot |= snapshot;
        if let Some(readers) = dependents.get(id) {
          for &reader in readers {
            if !queued[reader] {
              queued[reader] = true;
              queue.push_back(reader);
            }
          }
        }
      }
    }
  }
  fn source(&self, expr: &Expr) -> Source {
    match value_expr(expr) {
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
    let expr = value_expr(expr);
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
    if self.template_props.contains(&param.id.to_id())
      || crate::helper::should_render_as_jsx(result, &None)
    {
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
    if node.src.value == "jinge" && !node.type_only {
      for spec in &node.specifiers {
        if let ImportSpecifier::Named(n) = spec {
          if n.is_type_only {
            continue;
          }
          let intl = match &n.imported {
            Some(ModuleExportName::Ident(id)) => id.sym == "t",
            Some(ModuleExportName::Str(name)) => name.value == "t",
            None => n.local.sym == "t",
          };
          if intl {
            self.intl_imports.insert(n.local.to_id());
          }
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
    // Collect all declarations before inferring sources. Resolver Ids keep
    // forward references separate from lexical shadowing in other functions.
    for decl in &node.decls {
      let mut bindings = PatternBindings::default();
      decl.name.visit_with(&mut bindings);
      for id in &bindings.0 {
        let symbol = self.bindings.entry(id.clone()).or_insert(Symbol {
          source: Source::Unknown,
          constant: false,
          snapshot: false,
        });
        symbol.constant = node.kind == VarDeclKind::Const;
        self.scopes.last_mut().unwrap().push(id.clone());
      }
      if let Some(expr) = &decl.init {
        self.initializers.push(Initializer {
          bindings: bindings.0,
          expr: if matches!(value_expr(expr), Expr::Fn(_) | Expr::Arrow(_)) {
            None
          } else {
            Some(expr.clone())
          },
          destructured: !matches!(decl.name, Pat::Ident(_)),
        });
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

// Type-only wrappers do not change the runtime source of a value.
fn value_expr(mut expr: &Expr) -> &Expr {
  loop {
    expr = match expr {
      Expr::Paren(node) => &node.expr,
      Expr::TsAs(node) => &node.expr,
      Expr::TsTypeAssertion(node) => &node.expr,
      Expr::TsNonNull(node) => &node.expr,
      Expr::TsSatisfies(node) => &node.expr,
      Expr::TsInstantiation(node) => &node.expr,
      _ => return expr,
    };
  }
}

#[derive(Default)]
struct EagerBindings(HashSet<Id>);
impl Visit for EagerBindings {
  fn visit_ident(&mut self, id: &Ident) {
    self.0.insert(id.to_id());
  }
  fn visit_expr(&mut self, expr: &Expr) {
    match expr {
      Expr::Ident(id) => {
        self.0.insert(id.to_id());
      }
      Expr::Arrow(_) | Expr::Fn(_) | Expr::JSXElement(_) | Expr::JSXFragment(_) => {}
      _ => expr.visit_children_with(self),
    }
  }
  fn visit_function(&mut self, _: &Function) {}
  fn visit_ts_type(&mut self, _: &TsType) {}
  fn visit_getter_prop(&mut self, node: &GetterProp) {
    node.key.visit_with(self);
  }
  fn visit_setter_prop(&mut self, node: &SetterProp) {
    node.key.visit_with(self);
  }
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
  fn visit_getter_prop(&mut self, node: &GetterProp) {
    node.key.visit_with(self);
  }
  fn visit_setter_prop(&mut self, node: &SetterProp) {
    node.key.visit_with(self);
  }
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
    fn visit_getter_prop(&mut self, node: &GetterProp) {
      node.key.visit_with(self);
    }
    fn visit_setter_prop(&mut self, node: &SetterProp) {
      node.key.visit_with(self);
    }
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

pub fn is_intl_t(id: &Ident) -> bool {
  SYMBOLS.with(|symbols| symbols.borrow().intl_imports.contains(&id.to_id()))
}

/// Binding writes include assignment/destructuring/update/for-in/of targets,
/// but not member writes or a nested function's shadowing parameter.
pub(crate) fn binding_is_written(node: &ArrowFunctionBody, id: &Id) -> bool {
  let mut writes = WrittenBindings::default();
  node.visit_with(&mut writes);
  // `var item = ...` may reinitialize the very same parameter binding.
  struct Initializers(HashSet<Id>);
  impl Initializers {
    fn target<T: VisitWith<PatternBindings>>(&mut self, target: &T) {
      let mut bindings = PatternBindings::default();
      target.visit_with(&mut bindings);
      self.0.extend(bindings.0);
    }
  }
  impl Visit for Initializers {
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
      if node.init.is_some() {
        self.target(&node.name);
      }
      node.visit_children_with(self);
    }
    fn visit_for_of_stmt(&mut self, node: &ForOfStmt) {
      if let ForHead::VarDecl(decl) = &node.left {
        for binding in &decl.decls {
          self.target(&binding.name);
        }
      }
      node.visit_children_with(self);
    }
    fn visit_for_in_stmt(&mut self, node: &ForInStmt) {
      if let ForHead::VarDecl(decl) = &node.left {
        for binding in &decl.decls {
          self.target(&binding.name);
        }
      }
      node.visit_children_with(self);
    }
  }
  let mut initializers = Initializers(HashSet::new());
  node.visit_with(&mut initializers);
  writes.0.contains(id) || initializers.0.contains(id)
}

pub fn is_unresolved(id: &Ident) -> bool {
  SYMBOLS.with(|s| {
    s.borrow()
      .unresolved_mark
      .is_some_and(|mark| id.ctxt.has_mark(mark))
  })
}

pub fn validate_component_reference(id: &Ident) -> bool {
  let invalid = SYMBOLS.with(|s| s.borrow().invalid_component_aliases.contains(&id.to_id()));
  if invalid {
    crate::common::emit_error(
      id.span,
      "组件引用存在重新赋值，无法静态确定模板角色；请使用固定的 const 组件绑定",
    );
  }
  !invalid
}

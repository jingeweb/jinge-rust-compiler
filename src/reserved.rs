use swc_core::ecma::ast::*;
use swc_ecma_visit::{Visit, VisitWith};

/// Generated identifiers share this namespace because hygiene is intentionally disabled.
/// Check original runtime identifiers before stripping/rewriting; property keys and types
/// do not create runtime lexical bindings. Generated dictionaries may use only the fixed
/// intl parameter, not the rest of the compiler namespace.
pub struct ReservedNames {
  pub generated_intl: bool,
}

impl Visit for ReservedNames {
  fn visit_ident(&mut self, node: &Ident) {
    if node.sym.contains("$jg$") && !(self.generated_intl && node.sym == *"$jg$props") {
      crate::common::emit_error(node.span, "标识符不能包含编译器保留符号 $jg$，请更换名称");
    }
  }

  fn visit_import_decl(&mut self, node: &ImportDecl) {
    if !node.type_only {
      for spec in &node.specifiers {
        if !spec.is_type_only() {
          spec.local().visit_with(self);
        }
      }
    }
  }

  fn visit_named_export(&mut self, node: &NamedExport) {
    if !node.type_only && node.src.is_none() {
      for spec in &node.specifiers {
        if let ExportSpecifier::Named(spec) = spec {
          if !spec.is_type_only {
            spec.orig.visit_with(self);
          }
        }
      }
    }
  }

  fn visit_ts_type(&mut self, _: &TsType) {}
  fn visit_ts_expr_with_type_args(&mut self, _: &TsExprWithTypeArgs) {}
  fn visit_ts_type_ann(&mut self, _: &TsTypeAnn) {}
  fn visit_ts_type_param_decl(&mut self, _: &TsTypeParamDecl) {}
  fn visit_ts_interface_decl(&mut self, _: &TsInterfaceDecl) {}
  fn visit_ts_type_alias_decl(&mut self, _: &TsTypeAliasDecl) {}
  fn visit_ts_enum_member(&mut self, node: &TsEnumMember) {
    node.init.visit_with(self);
  }

  fn visit_decl(&mut self, node: &Decl) {
    match node {
      Decl::Var(n) if n.declare => {}
      Decl::Fn(n) if n.declare || n.function.body.is_none() => {}
      Decl::Class(n) if n.declare => {}
      Decl::TsEnum(n) if n.declare => {}
      Decl::TsModule(n) if n.declare || n.global => {}
      _ => node.visit_children_with(self),
    }
  }

  // Overloads and abstract method signatures have no runtime parameter bindings.
  fn visit_function(&mut self, node: &Function) {
    if node.body.is_some() {
      node.visit_children_with(self);
    }
  }
  fn visit_constructor(&mut self, node: &Constructor) {
    if node.body.is_some() {
      node.visit_children_with(self);
    }
  }

  fn visit_ts_import_equals_decl(&mut self, node: &TsImportEqualsDecl) {
    if !node.is_type_only {
      node.visit_children_with(self);
    }
  }

  fn visit_labeled_stmt(&mut self, node: &LabeledStmt) {
    node.body.visit_with(self);
  }
  fn visit_break_stmt(&mut self, _: &BreakStmt) {}
  fn visit_continue_stmt(&mut self, _: &ContinueStmt) {}
}

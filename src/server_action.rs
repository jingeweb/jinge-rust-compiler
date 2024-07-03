use swc_core::common::util::take::Take;
use swc_core::common::Spanned;
use swc_core::ecma::ast::{
  AssignPatProp, BindingIdent, CallExpr, Callee, Decl, Expr, ExprOrSpread, Ident, ImportDecl,
  ImportNamedSpecifier, ImportPhase, ImportSpecifier, Lit, Module, ModuleDecl, ModuleExportName,
  ModuleItem, ObjectPat, ObjectPatProp, Pat, Stmt, Str, VarDecl, VarDeclKind, VarDeclarator,
};

use crate::visitor::TransformVisitor;

pub fn transform_server_action(visitor: &mut TransformVisitor, n: &mut Module) {
  let mut replaced = false;
  let cfg = &visitor.config;

  n.body.iter_mut().for_each(|item| {
    let mut actions = vec![];
    let mut source = None;
    let mut namespace: Option<String> = None;

    if let ModuleItem::ModuleDecl(ModuleDecl::Import(imp)) = item {
      if imp.src.value.starts_with(&cfg.import_source) {
        // println!("\nSWC XXXXXXXX: {}\n", imp.src.value);
        if imp.src.value.len() != cfg.import_source.len() {
          namespace = Some(imp.src.value[cfg.import_source.len() + 1..].to_string())
        }
        for spec in imp.specifiers.iter_mut() {
          if let ImportSpecifier::Named(ref mut name) = spec {
            actions.push(name.local.take());
          }
        }
      }
      if !actions.is_empty() {
        source = Some(imp.src.take())
      }
    }

    if let Some(source) = source {
      let k = item.take();
      let x = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: k.span(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
          span: k.span(),
          name: Pat::Object(ObjectPat {
            span: k.span(),
            props: actions
              .iter_mut()
              .map(|act| {
                ObjectPatProp::Assign(AssignPatProp {
                  span: act.span(),
                  key: BindingIdent {
                    id: act.take(),
                    type_ann: None,
                  },
                  value: None,
                })
              })
              .collect(),
            optional: false,
            type_ann: None,
          }),
          init: Some(Box::new(if let Some(ns) = namespace.take() {
            Expr::Call(CallExpr {
              span: source.span(),
              type_args: None,
              callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                "__SERVER_ACTIONS_NS".into(),
                source.span(),
              )))),
              args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                  span: source.span(),
                  value: ns.into(),
                  raw: None,
                }))),
              }],
            })
          } else {
            Expr::Ident(Ident::new("__SERVER_ACTIONS".into(), source.span()))
          })),
          definite: false,
        }],
      }))));
      let _ = std::mem::replace(item, x);
      replaced = true;
    }
  });

  if replaced {
    let span = n.span();
    let mut new_body = Vec::with_capacity(n.body.len() + 1);
    new_body.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
      span,
      src: Box::new(Str::from(cfg.replace_source.as_str())),
      type_only: false,
      with: None,
      phase: ImportPhase::Evaluation,
      specifiers: vec![
        ImportSpecifier::Named(ImportNamedSpecifier {
          span,
          local: Ident::new("__SERVER_ACTIONS".into(), span),
          imported: Some(ModuleExportName::Ident(Ident::new(
            "rootProxy".into(),
            span,
          ))),
          is_type_only: false,
        }),
        ImportSpecifier::Named(ImportNamedSpecifier {
          span,
          local: Ident::new("__SERVER_ACTIONS_NS".into(), span),
          imported: Some(ModuleExportName::Ident(Ident::new(
            "getNamespaceProxy".into(),
            span,
          ))),
          is_type_only: false,
        }),
      ],
    })));
    new_body.append(&mut n.body);
    n.body = new_body
  }
}

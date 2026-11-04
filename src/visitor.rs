use swc_common::Spanned;
use swc_core::atoms::Wtf8Atom;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;
use swc_ecma_visit::{Visit, VisitMutWith, VisitWith};

use crate::ast::*;
use crate::common::*;
use crate::helper::*;
use crate::parser;
use crate::parser::slot::get_slot_name_from_member_expr;

pub struct TemplateTransformVisitor<'a> {
  changed: bool,
  pub parsed_components: &'a mut Vec<String>,
  pub intl_type: IntlType,
}
impl<'a> TemplateTransformVisitor<'a> {
  pub fn new(parsed_components: &'a mut Vec<String>, intl_type: IntlType) -> Self {
    Self {
      parsed_components,
      intl_type,
      changed: false,
    }
  }
  fn v_func(&mut self, fn_name: Option<&Ident>, expr: &mut Function, is_slot: bool) {
    if let Some(body) = &mut expr.body {
      let mut params: Vec<_> = expr.params.iter().map(|p| p.pat.clone()).collect();
      if self.v_func_body_stmts(fn_name, &mut body.stmts, &mut params, is_slot) {
        expr.params = params.into_iter().map(Param::from).collect();
      }
    };
  }
  fn v_func_body_stmts(
    &mut self,
    fn_name: Option<&Ident>,
    stmts: &mut [Stmt],
    params: &mut Vec<Pat>,
    is_slot: bool,
  ) -> bool {
    let mut changed = false;
    let mut root_host_arg = None;
    let last = stmts.iter().rposition(|s| !matches!(s, Stmt::Empty(_)));
    for (index, stmt) in stmts.iter_mut().enumerate().rev() {
      if Some(index) == last {
        let Stmt::Return(stmt) = stmt else {
          stmt.visit_mut_children_with(self);
          continue;
        };
        let Some(expr) = &mut stmt.arg else {
          continue;
        };
        let expr_ref = expr.as_ref();
        let mut should_parse_jsx = is_slot;
        if !should_parse_jsx {
          let props_arg = params.first().and_then(|p| match p {
            Pat::Ident(v) => Some(Wtf8Atom::new(v.sym.as_str())),
            _ => None,
          });
          should_parse_jsx = should_render_as_jsx(expr_ref, &props_arg);
        }
        if should_parse_jsx {
          if !is_slot {
            // 如果是最外层的函数组件，当有传递第二个参数 [H] 时，需要在第一行添加 const root_host$jg$ = [H]。
            // 这样对于 props.children 转成取 SLOTS 时，从 root_host$jg$ 取才不会有问题。
            if let Some(Pat::Ident(id)) = params.get(1) {
              root_host_arg.replace(id.id.clone());
            };
          }
          changed = self.v_return(fn_name, expr, params, is_slot);
        } else {
          expr.visit_mut_children_with(self);
        }
      } else {
        stmt.visit_mut_children_with(self);
      }
    }
    // if let Some(rh) = root_host_arg {
    //   // 如果是最外层的函数组件，当有传递第二个参数 [H] 时，需要在第一行添加 const root_host$jg$ = [H]。
    //   // 这样对于 props.children 转成取 SLOTS 时，从 root_host$jg$ 取才不会有问题。
    //   body.stmts.insert(
    //     0,
    //     ast_create_stmt_decl_const(JINGE_ROOT_HOST_IDENT.clone(), ast_create_expr_ident(rh)),
    //   );
    // }
    changed
  }
  fn v_arrow(&mut self, fn_name: Option<&Ident>, expr: &mut ArrowExpr, is_slot: bool) -> bool {
    match expr.body.as_mut() {
      ArrowFunctionBody::Expr(e) => {
        let mut should_parse_jsx = is_slot;
        let params = &mut expr.params;
        if !should_parse_jsx {
          let props_arg = params.first().and_then(|p| match p {
            Pat::Ident(v) => Some(Wtf8Atom::new(v.sym.as_str())),
            _ => None,
          });
          should_parse_jsx = should_render_as_jsx(e.as_ref(), &props_arg);
        }
        if should_parse_jsx {
          self.v_return(fn_name, e, params, is_slot)
        } else {
          false
        }
      }
      ArrowFunctionBody::FunctionBody(body) => {
        self.v_func_body_stmts(fn_name, &mut body.stmts, &mut expr.params, is_slot)
      }
    }
  }

  fn v_return(
    &mut self,
    fn_name: Option<&Ident>,
    expr: &mut Box<Expr>,
    params: &mut Vec<Pat>,
    is_slot: bool,
  ) -> bool {
    if let Some(replaced_expr) = self.v_return_parse(expr, params, is_slot) {
      *expr = replaced_expr;
      self.changed = true;
      if let Some(fn_name) = fn_name {
        self.parsed_components.push(fn_name.sym.to_string());
      }
      true
    } else {
      false
    }
  }

  fn v_return_parse(
    &mut self,
    expr: &mut Box<Expr>,
    params: &mut Vec<Pat>,
    is_slot: bool,
  ) -> Option<Box<Expr>> {
    const ERR: &str = "函数组件或 Slot 组件的参数不合法";
    let mut props_arg = None;
    if let Some(p) = params.first() {
      if let Pat::Ident(p) = p {
        props_arg.replace(Wtf8Atom::new(p.sym.as_str()));
      } else {
        emit_error(p.span(), ERR);
        return None;
      }
    } else {
      params.push(Pat::Ident(BindingIdent::from(JINGE_ATTR_IDENT.clone())));
    }
    let mut host_ident = None;
    if let Some(p) = params.get(1) {
      if let Pat::Ident(p) = p {
        if !p.sym.starts_with("host") {
          emit_error(
            p.span(),
            "函数组件的第二个参数必须是 host 或以 host 打头，确保已对第二个参数有充分理解",
          );
          return None;
        }
        host_ident.replace(p.sym.clone().into());
      } else {
        emit_error(p.span(), ERR);
        return None;
      }
    } else {
      // 如果是函数组件，则第二个参数默认添加 root_host$js$。
      // 如果是插槽函数，则第二个参数默认添加 host$jg$
      params.push(Pat::Ident(BindingIdent::from(if is_slot {
        JINGE_HOST_IDENT.clone()
      } else {
        JINGE_ROOT_HOST_IDENT.clone()
      })));
    }
    // println!("{:#?} {:#?}", props_arg, host_ident);
    let mut visitor = parser::TemplateParser::new(props_arg, host_ident.clone(), self.intl_type);
    if is_slot {
      visitor.push_context_with_host_ident(parser::Parent::Component, host_ident);
    }
    visitor.parse(expr.as_mut())
  }
}
impl VisitMut for TemplateTransformVisitor<'_> {
  fn visit_mut_function(&mut self, func: &mut Function) {
    self.v_func(None, func, false);
  }
  fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
    self.v_arrow(None, node, false);
  }
  fn visit_mut_object_lit(&mut self, node: &mut ObjectLit) {
    node.props.iter_mut().for_each(|prop| match prop {
      PropOrSpread::Prop(p) => match p.as_mut() {
        Prop::KeyValue(kv) => match &mut kv.key {
          PropName::Str(s) => {
            if s.value.starts_with("slot:") {
              let expr = &mut kv.value;
              match expr.as_mut() {
                Expr::Fn(expr) => {
                  self.v_func(None, &mut expr.function, true);
                  return;
                }
                Expr::Arrow(expr) => {
                  self.v_arrow(None, expr, true);
                  return;
                }
                Expr::Member(mem_expr) => {
                  // 如果 slot: 类型的属性，值是 member 表达式，则有可能是二次传递插槽。
                  // 比如 { 'slot:a': someVar['slot:b'] }
                  if let Some(slot_name) = get_slot_name_from_member_expr(mem_expr, &None) {
                    self.changed = true;
                    let pass_by = slot_name;
                    *expr = pass_by;
                    return;
                  }
                }
                Expr::OptChain(expr) => {
                  emit_error(expr.span, "对象插槽透传暂不支持 optional chain");
                  return;
                }
                _ => (),
              }
              // emit_error(s.span(), "slot:打头的属性代表插槽函数，属性值必须是函数");
              let mut params = vec![];
              if let Some(parsed_expr) = self.v_return_parse(expr, &mut params, true) {
                self.changed = true;
                *expr =
                  ast_create_expr_arrow_fn(params, Box::new(ArrowFunctionBody::Expr(parsed_expr)));
              }
            } else {
              kv.visit_mut_children_with(self);
            }
          }
          _ => prop.visit_mut_children_with(self),
        },
        _ => prop.visit_mut_children_with(self),
      },
      _ => prop.visit_mut_children_with(self),
    });
  }

  fn visit_mut_module(&mut self, n: &mut Module) {
    n.body.iter_mut().for_each(|item| match item {
      ModuleItem::ModuleDecl(decl) => match decl {
        ModuleDecl::ExportDecl(decl) => match &mut decl.decl {
          Decl::Fn(func) => self.v_func(Some(&func.ident), func.function.as_mut(), false),

          Decl::Var(decl) => decl.as_mut().decls.iter_mut().for_each(|decl| {
            if let Some(init) = &mut decl.init {
              let name = match &decl.name {
                Pat::Ident(id) => Some(&id.id),
                _ => {
                  emit_warn(decl.name.span(), "非常规命名的函数组件无法使用 HMR");
                  None
                }
              };
              match init.as_mut() {
                Expr::Fn(func) => self.v_func(name, func.function.as_mut(), false),
                Expr::Arrow(func) => {
                  self.v_arrow(name, func, false);
                }
                _ => init.visit_mut_children_with(self),
              }
            }
          }),
          _ => decl.visit_mut_children_with(self),
        },
        ModuleDecl::ExportDefaultDecl(decl) => match &mut decl.decl {
          DefaultDecl::Fn(func) => self.v_func(
            if let Some(n) = &func.ident {
              Some(n)
            } else {
              emit_warn(func.span(), "匿名函数组件无法使用 HMR");
              None
            },
            func.function.as_mut(),
            false,
          ),
          _ => decl.visit_mut_children_with(self),
        },
        _ => decl.visit_mut_children_with(self),
      },
      ModuleItem::Stmt(stmt) => match stmt {
        Stmt::Decl(decl) => match decl {
          Decl::Fn(decl) => self.v_func(Some(&decl.ident), decl.function.as_mut(), false),
          Decl::Var(decl) => decl.decls.iter_mut().for_each(|decl| {
            if let Some(init) = decl.init.as_mut() {
              let name = match &decl.name {
                Pat::Ident(id) => Some(&id.id),
                _ => None,
              };
              match init.as_mut() {
                Expr::Fn(func) => self.v_func(name, func.function.as_mut(), false),
                Expr::Arrow(func) => {
                  self.v_arrow(name, func, false);
                }
                _ => init.visit_mut_children_with(self),
              }
            }
          }),
          _ => decl.visit_mut_children_with(self),
        },
        _ => stmt.visit_mut_children_with(self),
      },
    });

    if self.changed {
      let mut new_items = Vec::with_capacity(n.body.len() + 1);
      let mut imports = JINGE_IMPORT_MODULE_ITEM.clone();
      let mut comment_usage = CommentUsage(false);
      n.visit_with(&mut comment_usage);
      if comment_usage.0
        && let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &mut imports
      {
        import
          .specifiers
          .push(ImportSpecifier::Named(ImportNamedSpecifier {
            span: swc_common::DUMMY_SP,
            local: JINGE_IMPORT_CREATE_COMMENT.local(),
            imported: Some(ModuleExportName::Ident(
              JINGE_IMPORT_CREATE_COMMENT.imported(),
            )),
            is_type_only: false,
          }));
      }
      let mut method_usage = MethodUsage(false);
      n.visit_with(&mut method_usage);
      if method_usage.0
        && let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &mut imports
      {
        import
          .specifiers
          .push(ImportSpecifier::Named(ImportNamedSpecifier {
            span: swc_common::DUMMY_SP,
            local: JINGE_IMPORT_METHOD_PATH_WATCHER.local(),
            imported: Some(ModuleExportName::Ident(
              JINGE_IMPORT_METHOD_PATH_WATCHER.imported(),
            )),
            is_type_only: false,
          }));
      }
      new_items.push(imports);
      new_items.append(&mut n.body);

      n.body = new_items;
    }
  }

  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    let IntlType::Enabled(drop_default_text) = self.intl_type else {
      node.visit_mut_children_with(self);
      return;
    };

    let Callee::Expr(callee) = &node.callee else {
      node.visit_mut_children_with(self);
      return;
    };

    if !matches!(callee.as_ref(), Expr::Ident(name) if crate::scope::is_intl_t(name)) {
      node.visit_mut_children_with(self);
      return;
    }

    let args = parse_intl_call_args(&node.args, drop_default_text);
    node.args = args;
  }
}

/// Plain .ts internationalization must not run the component visitor.
pub struct IntlTransformVisitor {
  pub intl_type: IntlType,
}
impl VisitMut for IntlTransformVisitor {
  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    node.visit_mut_children_with(self);
    if let IntlType::Enabled(drop) = self.intl_type
      && matches!(&node.callee, Callee::Expr(c) if matches!(&**c, Expr::Ident(id) if crate::scope::is_intl_t(id)))
    {
      node.args = parse_intl_call_args(&node.args, drop);
    }
  }
}

/// Any JSX left after template conversion is unsupported syntax, never valid JS.
pub struct RemainingJsx;
impl swc_ecma_visit::Visit for RemainingJsx {
  fn visit_jsx_element(&mut self, node: &JSXElement) {
    emit_error(
      node.span,
      "不支持此位置的 JSX（包括提前 return）；请使用最终模板 return",
    );
  }
  fn visit_jsx_fragment(&mut self, node: &JSXFragment) {
    emit_error(
      node.span,
      "不支持此位置的 JSX Fragment；请使用最终模板 return",
    );
  }
}

// Keep the common import list unchanged for templates that need no empty-slot anchor.
struct CommentUsage(bool);
impl Visit for CommentUsage {
  fn visit_call_expr(&mut self, node: &CallExpr) {
    if matches!(&node.callee, Callee::Expr(callee) if matches!(&**callee, Expr::Ident(id) if id.span.is_dummy() && id.sym == JINGE_IMPORT_CREATE_COMMENT.local().sym))
    {
      self.0 = true;
    }
    node.visit_children_with(self);
  }
}

struct MethodUsage(bool);
impl Visit for MethodUsage {
  fn visit_call_expr(&mut self, node: &CallExpr) {
    if matches!(&node.callee, Callee::Expr(callee) if matches!(&**callee, Expr::Ident(id) if id.span.is_dummy() && id.sym == JINGE_IMPORT_METHOD_PATH_WATCHER.local().sym))
    {
      self.0 = true;
    }
    node.visit_children_with(self);
  }
}

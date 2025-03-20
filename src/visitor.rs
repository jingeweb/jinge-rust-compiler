use swc_common::Spanned;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;
use swc_ecma_visit::VisitMutWith;

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
      if self.v_func_body(fn_name, body, &mut params, is_slot) {
        expr.params = params.into_iter().map(|p| Param::from(p)).collect();
      }
    };
  }
  fn v_func_body(
    &mut self,
    fn_name: Option<&Ident>,
    body: &mut BlockStmt,
    params: &mut Vec<Pat>,
    is_slot: bool,
  ) -> bool {
    let mut changed = false;
    let mut root_host_arg = None;
    for (index, stmt) in body.stmts.iter_mut().rev().enumerate() {
      if index == 0 {
        let Stmt::Return(stmt) = stmt else {
          stmt.visit_mut_children_with(self);
          continue;
        };
        let Some(expr) = &mut stmt.arg else {
          continue;
        };
        if is_slot || has_jsx(expr.as_ref()) {
          if !is_slot {
            // 如果是最外层的函数组件，当有传递第二个参数 [H] 时，需要在第一行添加 const root_host$jg$ = [H]。
            // 这样对于 props.children 转成取 SLOTS 时，从 root_host$jg$ 取才不会有问题。
            match params.get(1) {
              Some(Pat::Ident(id)) => {
                root_host_arg.replace(id.id.clone());
              }
              _ => (),
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
      BlockStmtOrExpr::Expr(e) => {
        if is_slot || has_jsx(e.as_ref()) {
          self.v_return(fn_name, e, &mut expr.params, is_slot)
        } else {
          false
        }
      }
      BlockStmtOrExpr::BlockStmt(body) => {
        self.v_func_body(fn_name, body, &mut expr.params, is_slot)
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
    if let Some(p) = params.get(0) {
      if let Pat::Ident(p) = p {
        props_arg.replace(p.sym.clone());
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
    let mut visitor =
      parser::TemplateParser::new(props_arg, host_ident.clone(), self.intl_type.clone());
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
                Expr::OptChain(_) => todo!("支持 optional-chain"),
                _ => (),
              }
              // emit_error(s.span(), "slot:打头的属性代表插槽函数，属性值必须是函数");
              let mut params = vec![];
              if let Some(parsed_expr) = self.v_return_parse(expr, &mut params, true) {
                *expr =
                  ast_create_expr_arrow_fn(params, Box::new(BlockStmtOrExpr::Expr(parsed_expr)));
              }
              return;
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
                  emit_error(decl.name.span(), "警告：非常规命令的函数组件无法使用 HMR");
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
              emit_error(func.span(), "警告：匿名函数组件无法使用 HMR");
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
      new_items.push(JINGE_IMPORT_MODULE_ITEM.clone());
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

    if !matches!(callee.as_ref(), Expr::Ident(name) if JINGE_T.eq(&name.sym)) {
      node.visit_mut_children_with(self);
      return;
    }

    parse_intl_call(node, drop_default_text);
  }
}

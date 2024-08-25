use std::rc::Rc;

use hashbrown::HashSet;
use swc_common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    AssignExpr, AssignOp, AssignTarget, BlockStmt, BlockStmtOrExpr, CallExpr, Callee,
    ComputedPropName, Expr, ExprOrSpread, ExprStmt, IdentName, KeyValueProp, Lit, MemberExpr,
    MemberProp, ObjectLit, Pat, Prop, PropName, PropOrSpread, ReturnStmt, SimpleAssignTarget, Stmt,
  },
};

use crate::{
  ast::{
    ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
    ast_create_expr_member, ast_create_expr_this, ast_create_id_of_container,
    ast_create_stmt_decl_const,
  },
  parser::{
    expr::ExprVisitor, tpl::tpl_watch_and_render, JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_V_IDENT,
  },
};

use super::{
  emit_error, expr::ExprParseResult, tpl::tpl_push_el_code, TemplateParser, JINGE_EL_IDENT,
  JINGE_IMPORT_CONTEXT, JINGE_IMPORT_DEFAULT_SLOT, JINGE_IMPORT_NEW_SLOT_RENDER_COM,
  JINGE_IMPORT_SLOTS, JINGE_RENDER, JINGE_SLOTS,
};

#[derive(Debug)]
enum Slot {
  None,
  Default,
  Named(Atom),
}
fn get_slot(expr: &CallExpr) -> Slot {
  let Callee::Expr(expr) = &expr.callee else {
    return Slot::None;
  };
  let Expr::Member(expr) = expr.as_ref() else {
    return Slot::None;
  };
  // println!("{:#?}", expr);
  match expr.obj.as_ref() {
    Expr::This(_) => {
      let MemberProp::Ident(prop) = &expr.prop else {
        return Slot::None;
      };
      if JINGE_SLOTS.eq(&prop.sym) {
        Slot::Default
      } else {
        Slot::None
      }
    }
    Expr::Member(expr2) => match expr2.obj.as_ref() {
      Expr::This(_) => {
        let MemberProp::Ident(prop) = &expr2.prop else {
          return Slot::None;
        };
        if !(JINGE_SLOTS.eq(&prop.sym)) {
          return Slot::None;
        }
        let MemberProp::Ident(prop) = &expr.prop else {
          return Slot::None;
        };
        Slot::Named(prop.sym.clone())
      }
      _ => Slot::None,
    },
    _ => Slot::None,
  }
}

struct SlotVm {
  pub const_props: Vec<(PropName, Box<Expr>)>,
  pub watch_props: Vec<(PropName, ExprParseResult)>,
}
fn parse_slot_arg(expr: &CallExpr) -> SlotVm {
  let mut vm = SlotVm {
    const_props: vec![],
    watch_props: vec![],
  };

  if expr.args.len() > 1 {
    emit_error(
      expr.span(),
      "警告：slot 渲染函数的第2个及之后的参数将被忽略。",
    );
  }
  let Some(arg) = expr.args.first() else {
    return vm;
  };
  if arg.spread.is_some() {
    emit_error(expr.span(), "Slot 渲染函数的参数不支持 ... 解构写法。");
    return vm;
  }
  let Expr::Object(arg) = arg.expr.as_ref() else {
    emit_error(expr.span(), "Slot 渲染参数必须是 key-value 类型的 Object。");
    return vm;
  };

  for prop in arg.props.iter() {
    let PropOrSpread::Prop(prop) = prop else {
      emit_error(prop.span(), "Slot 渲染参数不支持 ... 解构写法。");
      return vm;
    };
    let Prop::KeyValue(kv) = prop.as_ref() else {
      emit_error(prop.span(), "Slot 渲染参数必须是 key-value 类型的 Object。");
      return vm;
    };

    match kv.value.as_ref() {
      Expr::JSXElement(_)
      | Expr::JSXEmpty(_)
      | Expr::JSXFragment(_)
      | Expr::JSXMember(_)
      | Expr::JSXNamespacedName(_) => {
        emit_error(kv.value.span(), "不支持 JSX 元素作为属性值");
      }
      Expr::Lit(val) => {
        vm.const_props
          .push((kv.key.clone(), Box::new(Expr::Lit(val.clone()))));
      }
      Expr::Fn(_) | Expr::Arrow(_) => {
        let mut set: HashSet<Atom> = HashSet::new();
        match kv.value.as_ref() {
          Expr::Fn(e) => e.function.params.iter().for_each(|p| {
            if let Pat::Ident(id) = &p.pat {
              set.insert(id.sym.clone());
            }
          }),
          Expr::Arrow(e) => e.params.iter().for_each(|p| {
            if let Pat::Ident(id) = p {
              set.insert(id.sym.clone());
            }
          }),
          _ => (),
        }
        let r = ExprVisitor::new_with_exclude_roots(if set.is_empty() {
          None
        } else {
          Some(Rc::new(set))
        })
        .parse(kv.value.as_ref());
        match r {
          ExprParseResult::None => {
            vm.const_props.push((kv.key.clone(), kv.value.clone()));
          }
          _ => vm.watch_props.push((kv.key.clone(), r)),
        }
      }
      _ => {
        let r = ExprVisitor::new().parse(kv.value.as_ref());
        match r {
          ExprParseResult::None => {
            vm.const_props.push((kv.key.clone(), kv.value.clone()));
          }
          _ => vm.watch_props.push((kv.key.clone(), r)),
        }
      }
    }
  }
  vm
}

impl TemplateParser {
  fn transform_slot(&mut self, expr: &CallExpr, slot_name: Option<Atom>) {
    let slot_arg_vm = parse_slot_arg(expr);

    let root_container = self.context.root_container;
    let args = vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_ATTR_IDENT.clone())),
      ast_create_arg_expr(ast_create_expr_member(
        ast_create_id_of_container(root_container),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
        }),
      )),
      ast_create_arg_expr(ast_create_expr_member(
        ast_create_expr_member(
          ast_create_expr_this(),
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_SLOTS.local()),
          }),
        ),
        if let Some(slot_name) = slot_name {
          MemberProp::Ident(IdentName::from(slot_name))
        } else {
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_DEFAULT_SLOT.local()),
          })
        },
      )),
    ];
    let mut stmts: Vec<Stmt> = vec![ast_create_stmt_decl_const(
      JINGE_ATTR_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_VM.local()),
        vec![ast_create_arg_expr(Box::new(Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: slot_arg_vm
            .const_props
            .into_iter()
            .map(|(prop, value)| {
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp { key: prop, value })))
            })
            .collect(),
        })))],
      ),
    )];

    slot_arg_vm
      .watch_props
      .into_iter()
      .for_each(|(attr_name, watch_expr)| {
        let set_fn = Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          op: AssignOp::Assign,
          left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
            span: DUMMY_SP,
            obj: ast_create_expr_ident(JINGE_ATTR_IDENT.clone()),
            prop: match attr_name {
              PropName::Ident(id) => MemberProp::Ident(id),
              PropName::Computed(e) => MemberProp::Computed(e),
              PropName::Num(x) => MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::Num(x))),
              }),
              PropName::Str(x) => MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::Str(x))),
              }),
              PropName::BigInt(x) => MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::BigInt(x))),
              }),
            },
          })),
          right: ast_create_expr_ident(JINGE_V_IDENT.clone()),
        }));

        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_render(set_fn, watch_expr, self.context.root_container),
        }));
      });

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_NEW_SLOT_RENDER_COM.local()),
        args,
      ),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), root_container),
    }));

    stmts.push(Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(ast_create_expr_call(
        ast_create_expr_member(
          ast_create_expr_ident(JINGE_EL_IDENT.clone()),
          MemberProp::Ident(IdentName::from(JINGE_RENDER.clone())),
        ),
        vec![],
      )),
    }));

    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
        spread: Some(DUMMY_SP),
        expr: ast_create_expr_call(
          ast_create_expr_arrow_fn(
            vec![],
            Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
              span: DUMMY_SP,
              ctxt: SyntaxContext::empty(),
              stmts,
            })),
          ),
          vec![],
        ),
      });
  }
  pub fn parse_slot_call_expr(&mut self, expr: &CallExpr) -> bool {
    match get_slot(expr) {
      Slot::None => false,
      Slot::Default => {
        self.transform_slot(expr, None);
        true
      }
      Slot::Named(n) => {
        self.transform_slot(expr, Some(n));
        true
      }
    }
  }
}

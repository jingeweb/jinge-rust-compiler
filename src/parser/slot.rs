use std::rc::Rc;

use hashbrown::HashSet;
use swc_common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    AssignExpr, AssignOp, AssignTarget, BlockStmt, BlockStmtOrExpr, CallExpr, Callee,
    ComputedPropName, Expr, ExprOrSpread, ExprStmt, Ident, IdentName, KeyValueProp, Lit,
    MemberExpr, MemberProp, ObjectLit, Pat, Prop, PropName, PropOrSpread, ReturnStmt,
    SimpleAssignTarget, Stmt,
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
  emit_error, expr::ExprParseResult, tpl::tpl_push_el_code, TemplateParser, JINGE_CHILDREN,
  JINGE_EL_IDENT, JINGE_IMPORT_CONTEXT, JINGE_IMPORT_DEFAULT_SLOT,
  JINGE_IMPORT_NEW_COM_DEFAULT_SLOT, JINGE_IMPORT_RENDER_SLOT, JINGE_IMPORT_SLOTS, JINGE_SLOTS,
};

#[derive(Debug)]
enum Slot {
  None,
  Default,
  Named(Atom),
}
fn get_slot(expr: &MemberExpr, props_arg: &Atom) -> Slot {
  // println!("{:#?}", expr);
  match expr.obj.as_ref() {
    Expr::Ident(id) if props_arg.eq(&id.sym) => {
      let MemberProp::Ident(prop) = &expr.prop else {
        return Slot::None;
      };
      if JINGE_CHILDREN.eq(&prop.sym) || JINGE_SLOTS.eq(&prop.sym) {
        Slot::Default
      } else {
        Slot::None
      }
    }
    Expr::Member(expr2) => match expr2.obj.as_ref() {
      Expr::Ident(id) if props_arg.eq(&id.sym) => {
        let MemberProp::Ident(prop) = &expr2.prop else {
          return Slot::None;
        };
        if !(JINGE_CHILDREN.eq(&prop.sym)) && !(JINGE_SLOTS.eq(&prop.sym)) {
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
  pub spread_prop: Option<Ident>,
}
fn parse_slot_arg(args: &Vec<ExprOrSpread>) -> SlotVm {
  let mut vm = SlotVm {
    const_props: vec![],
    watch_props: vec![],
    spread_prop: None,
  };

  if args.len() > 1 {
    emit_error(
      args[1].span(),
      "警告：slot 渲染函数的只允许一个参数，该参数应该是具备双向绑定属性的 ViewModel。是否忘了使用 object 包裹这几个参数？",
    );
    return vm;
  }
  let Some(arg) = args.first() else {
    return vm;
  };

  if arg.spread.is_some() {
    emit_error(arg.span(), "Slot 渲染函数的参数不支持 ... 解构数组的写法。");
    return vm;
  }
  let arg = match arg.expr.as_ref() {
    Expr::Ident(id) => {
      let msg = format!("Slot 渲染参数应该是具备双向绑定属性的 ViewModel。是否忘了使用 object 包裹 {0}？如果就是想透传该 ViewModel 作为 Slot 参数，可使用 {{...{0}}} 的写法。", id.sym);
      emit_error(arg.span(), &msg);
      return vm;
    }
    Expr::Object(arg) => arg,
    _ => {
      emit_error(arg.span(), "Slot 渲染参数必须是 key-value 类型的 Object。");
      return vm;
    }
  };

  for prop in arg.props.iter() {
    match prop {
      PropOrSpread::Spread(s) => {
        let Expr::Ident(id) = s.expr.as_ref() else {
          emit_error(s.span(), "解构写法...后必须是 Ident");
          return vm;
        };
        if vm.spread_prop.is_some() {
          emit_error(s.span(), "解构写法透传属性只能出现一次");
        } else {
          vm.spread_prop.replace(id.clone());
        }
      }
      PropOrSpread::Prop(prop) => {
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
    }
  }

  if vm.spread_prop.is_some() && (!vm.const_props.is_empty() || !vm.watch_props.is_empty()) {
    let id = vm.spread_prop.take();
    emit_error(id.span(), "解构写法透传属性只能出现一次");
  }

  vm
}

impl TemplateParser {
  fn transform_slot(&mut self, slot_name: Option<Atom>, slot_args: Option<&Vec<ExprOrSpread>>) {
    let mut stmts = vec![];

    let slot_vm_id =
      slot_args.and_then(|slot_args| self.transform_slot_args(slot_args, &mut stmts));

    let root_container = self.context.root_container;

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_NEW_COM_DEFAULT_SLOT.local()),
        vec![ast_create_arg_expr(ast_create_expr_member(
          ast_create_id_of_container(root_container),
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
          }),
        ))],
      ),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), root_container),
    }));

    let mut args = vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
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
    if let Some(id) = slot_vm_id {
      args.push(ast_create_arg_expr(ast_create_expr_ident(id)));
    }
    stmts.push(Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_RENDER_SLOT.local()),
        args,
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
  fn transform_slot_args(
    &mut self,
    args: &Vec<ExprOrSpread>,
    stmts: &mut Vec<Stmt>,
  ) -> Option<Ident> {
    let mut slot_arg_vm = parse_slot_arg(args);

    let has_slot_vm = !slot_arg_vm.const_props.is_empty() || !slot_arg_vm.watch_props.is_empty();
    if has_slot_vm {
      let slot_props = Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: slot_arg_vm
          .const_props
          .into_iter()
          .map(|(prop, value)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp { key: prop, value })))
          })
          .collect(),
      }));
      stmts.push(ast_create_stmt_decl_const(
        JINGE_ATTR_IDENT.clone(),
        if slot_arg_vm.watch_props.is_empty() {
          slot_props
        } else {
          ast_create_expr_call(
            ast_create_expr_ident(JINGE_IMPORT_VM.local()),
            vec![ast_create_arg_expr(slot_props)],
          )
        },
      ));
    }
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

    if has_slot_vm {
      return Some(JINGE_ATTR_IDENT.clone());
    }

    slot_arg_vm.spread_prop.take()
  }
  pub fn parse_slot_mem_expr(
    &mut self,
    expr: &MemberExpr,
    slot_args: Option<&Vec<ExprOrSpread>>,
  ) -> bool {
    let Some(props_arg) = &self.props_arg else {
      return false;
    };
    match get_slot(expr, props_arg) {
      Slot::None => false,
      Slot::Default => {
        self.transform_slot(None, slot_args);
        true
      }
      Slot::Named(n) => {
        self.transform_slot(Some(n), slot_args);
        true
      }
    }
  }
  pub fn parse_slot_call_expr(&mut self, expr: &CallExpr) -> bool {
    let Some(props_arg) = &self.props_arg else {
      return false;
    };
    let Callee::Expr(e) = &expr.callee else {
      return false;
    };
    let Expr::Member(e) = e.as_ref() else {
      return false;
    };
    match get_slot(e, props_arg) {
      Slot::None => false,
      Slot::Default => {
        self.transform_slot(None, Some(&expr.args));
        true
      }
      Slot::Named(n) => {
        self.transform_slot(Some(n), Some(&expr.args));
        true
      }
    }
  }
}

use swc_common::{DUMMY_SP, Spanned, SyntaxContext};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    ArrayLit, AssignExpr, AssignOp, AssignTarget, BinExpr, BlockStmt, BlockStmtOrExpr,
    ComputedPropName, CondExpr, Expr, ExprOrSpread, ExprStmt, Ident, IdentName, KeyValueProp, Lit,
    MemberExpr, MemberProp, ObjectLit, OptChainBase, Prop, PropName, PropOrSpread, ReturnStmt,
    SimpleAssignTarget, Stmt,
  },
};
use swc_ecma_visit::Visit;

use crate::{
  ast::{
    ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
    ast_create_expr_member, ast_create_expr_this, ast_create_id_of_container,
    ast_create_stmt_decl_const,
  },
  common::emit_warn,
  parser::{
    JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_V_IDENT, expr::ExprVisitor, tpl::tpl_watch_and_render,
  },
};

use super::{
  JINGE_CHILDREN, JINGE_EL_IDENT, JINGE_IMPORT_CONTEXT, JINGE_IMPORT_DEFAULT_SLOT,
  JINGE_IMPORT_NEW_COM_DEFAULT_SLOT, JINGE_IMPORT_RENDER_SLOT, JINGE_IMPORT_SLOTS, TemplateParser,
  emit_error, expr::ExprParseResult, tpl::tpl_push_el_code,
};

#[derive(Debug)]
enum Slot {
  None,
  Default,
  Named(Atom),
}

fn get_slot(expr: &MemberExpr, props_arg: &Atom) -> Slot {
  let Expr::Ident(id) = expr.obj.as_ref() else {
    return Slot::None;
  };
  if !id.sym.eq(props_arg) {
    return Slot::None;
  }
  match &expr.prop {
    MemberProp::Ident(id) => {
      if JINGE_CHILDREN.eq(&id.sym) {
        Slot::Default
      } else {
        Slot::None
      }
    }
    MemberProp::Computed(e) => match e.expr.as_ref() {
      Expr::Lit(id) => match id {
        Lit::Str(id) => {
          if JINGE_CHILDREN.eq(&id.value) {
            Slot::Default
          } else if id.value.starts_with("slot:") {
            Slot::Named(id.value[5..].into())
          } else {
            Slot::None
          }
        }
        _ => Slot::None,
      },
      _ => Slot::None,
    },
    _ => Slot::None,
  }
}

#[inline]
fn exprorspread_to_expr(expr: ExprOrSpread) -> Box<Expr> {
  if expr.spread.is_some() {
    expr.expr
  } else {
    Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![Some(expr)],
    }))
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
      "警告：Slot 渲染函数的只允许一个参数，该参数应该是具备双向绑定属性的 ViewModel。是否忘了使用 object 包裹这几个参数？",
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
      let msg = format!(
        "Slot 渲染参数应该是具备双向绑定属性的 ViewModel。是否忘了使用 object 包裹 {0}？如果就是想透传该 ViewModel 作为 Slot 参数，可使用 {{...{0}}} 的写法。",
        id.sym
      );
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
            emit_error(kv.value.span(), "不支持 JSX 元素作为插槽的传递数据");
          }
          Expr::Lit(val) => {
            vm.const_props
              .push((kv.key.clone(), Box::new(Expr::Lit(val.clone()))));
          }
          Expr::Fn(_) | Expr::Arrow(_) => {
            // let mut set: HashSet<Atom> = HashSet::new();
            // match kv.value.as_ref() {
            //   Expr::Fn(e) => e.function.params.iter().for_each(|p| {
            //     if let Pat::Ident(id) = &p.pat {
            //       set.insert(id.sym.clone());
            //     }
            //   }),
            //   Expr::Arrow(e) => e.params.iter().for_each(|p| {
            //     if let Pat::Ident(id) = p {
            //       set.insert(id.sym.clone());
            //     }
            //   }),
            //   _ => (),
            // }
            // println!("XXXX {:#?}", set);
            // let r = ExprVisitor::new_with_exclude_roots(if set.is_empty() {
            //   None
            // } else {
            //   Some(Rc::new(set))
            // })
            // .parse(kv.value.as_ref());
            // match r {
            //   ExprParseResult::None => {
            //     vm.const_props.push((kv.key.clone(), kv.value.clone()));
            //   }
            //   _ => vm.watch_props.push((kv.key.clone(), r)),
            // }
            emit_error(kv.value.span(), "不支持函数作为插槽的传递数据");
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

fn slot_name_to_mem(slot_name: Option<Atom>) -> Box<Expr> {
  ast_create_expr_member(
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
  )
}

impl TemplateParser {
  fn transform_slot_to_render_fn(
    &mut self,
    slot_name: Option<Atom>,
    slot_args: Option<&Vec<ExprOrSpread>>,
  ) -> Box<Expr> {
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
      ast_create_arg_expr(slot_name_to_mem(slot_name)),
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

    ast_create_expr_call(
      ast_create_expr_arrow_fn(
        vec![],
        Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          stmts,
        })),
      ),
      vec![],
    )
  }
  #[inline]
  fn transform_slot(&mut self, slot_name: Option<Atom>, slot_args: Option<&Vec<ExprOrSpread>>) {
    let render_fn_expr = self.transform_slot_to_render_fn(slot_name, slot_args);
    self.push_expression_with_spread(render_fn_expr);
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
    let slot_name = match get_slot(expr, props_arg) {
      Slot::None => return false,
      Slot::Default => None,
      Slot::Named(n) => Some(n),
    };
    self.transform_slot(slot_name, slot_args);
    true
  }
  pub fn parse_slot_call_expr(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    let Some(props_arg) = &self.props_arg else {
      return false;
    };

    let maybe_slot_expr = match callee {
      Expr::Member(e) => e,
      Expr::OptChain(oc) => {
        if let OptChainBase::Member(m) = oc.base.as_ref() {
          m
        } else {
          return false;
        }
      }
      _ => return false,
    };
    let slot_name = match get_slot(maybe_slot_expr, props_arg) {
      Slot::None => return false,
      Slot::Default => None,
      Slot::Named(n) => Some(n),
    };
    self.transform_slot(slot_name, Some(args));
    true
  }

  fn get_bin_expr_slot_name(&self, expr: &Expr) -> Option<Option<Atom>> {
    let Some(props_arg) = &self.props_arg else {
      return None;
    };
    let Expr::Member(mem) = expr else {
      return None;
    };

    match get_slot(mem, props_arg) {
      Slot::None => None,
      Slot::Default => Some(None),
      Slot::Named(n) => Some(Some(n)),
    }
  }
  fn parse_expr_to_render_fn(&mut self, expr: &Expr) -> Option<ExprOrSpread> {
    self.push_context(self.context.parent, self.context.root_container);
    self.visit_expr(expr);
    let mut context = self.pop_context();
    context.slots.pop().and_then(|mut s| s.expressions.pop())
  }
  pub fn parse_cond_slot(&mut self, expr: &CondExpr) -> bool {
    let Some(slot_name) = self.get_bin_expr_slot_name(&expr.test) else {
      return false;
    };
    let Some(conds_render_fn) = self.parse_expr_to_render_fn(&expr.cons) else {
      emit_warn(expr.span(), "unexpected");
      return true;
    };
    let Some(alt_render_fn) = self.parse_expr_to_render_fn(&expr.alt) else {
      emit_warn(expr.span(), "unexpected");
      return true;
    };
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,

      test: slot_name_to_mem(slot_name),
      cons: exprorspread_to_expr(conds_render_fn),
      alt: exprorspread_to_expr(alt_render_fn),
    })));
    true
  }

  pub fn parse_logic_and_slot(&mut self, expr: &BinExpr) -> bool {
    let Some(slot_name) = self.get_bin_expr_slot_name(&expr.left) else {
      return false;
    };
    let Some(render_fn) = self.parse_expr_to_render_fn(&expr.right) else {
      emit_warn(expr.span(), "unexpected");
      return true;
    };
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name_to_mem(slot_name),
      cons: exprorspread_to_expr(render_fn),
      alt: Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![],
      })),
    })));
    true
  }

  pub fn parse_nullish_coalescing_slot(&mut self, expr: &BinExpr) -> bool {
    let Some(slot_name) = self.get_bin_expr_slot_name(&expr.left) else {
      return false;
    };
    let Some(default_slot) = self.parse_expr_to_render_fn(&expr.right) else {
      emit_warn(expr.span(), "unexpected");
      return true;
    };
    let render_fn = self.transform_slot_to_render_fn(slot_name.clone(), None);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name_to_mem(slot_name),
      cons: render_fn,
      alt: exprorspread_to_expr(default_slot),
    })));
    true
  }
}

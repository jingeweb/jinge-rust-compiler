use swc_common::{DUMMY_SP, Spanned, SyntaxContext};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    ArrayLit, AssignExpr, AssignOp, AssignTarget, BinExpr, BlockStmt, BlockStmtOrExpr,
    ComputedPropName, CondExpr, Expr, ExprOrSpread, ExprStmt, Ident, IdentName, KeyValueProp, Lit,
    MemberExpr, MemberProp, Null, ObjectLit, OptChainBase, Prop, PropName, PropOrSpread,
    ReturnStmt, SimpleAssignTarget, Stmt,
  },
};
use swc_ecma_visit::Visit;

use crate::{
  ast::*,
  common::JINGE_ROOT_HOST_IDENT,
  parser::{
    JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_V_IDENT, expr::ExprVisitor, tpl::tpl_watch_and_render,
  },
};

use super::{
  JINGE_CHILDREN, JINGE_EL_IDENT, JINGE_IMPORT_CONTEXT, JINGE_IMPORT_DEFAULT_SLOT,
  JINGE_IMPORT_NEW_COM_DEFAULT_SLOT, JINGE_IMPORT_RENDER_SLOT, JINGE_IMPORT_SLOTS, TemplateParser,
  emit_error, expr::ExprParseResult, tpl::tpl_push_el_code,
};

#[derive(Debug, Clone)]
pub enum SlotName {
  None,
  /// props.children 这种写法的默认 slot
  Default,
  /// props['slot:xx'] 这种写法的命名 slot
  Named(Atom),
  /// props.somevar['slot:xx'] 这种写法的由普通属性参数传递的 slot
  Expr(Box<Expr>),
}

pub fn get_slot_name_from_member_expr(expr: &MemberExpr, props_arg: &Option<Atom>) -> SlotName {
  match &expr.prop {
    MemberProp::Ident(id) => {
      if JINGE_CHILDREN.eq(&id.sym) {
        if let Some(props_arg) = props_arg {
          if matches!(expr.obj.as_ref(), Expr::Ident(id) if id.sym.eq(props_arg)) {
            SlotName::Default
          } else {
            SlotName::None
          }
        } else {
          SlotName::None
        }
      } else {
        SlotName::None
      }
    }
    MemberProp::Computed(e) => match e.expr.as_ref() {
      Expr::Lit(id) => match id {
        Lit::Str(id) => {
          if JINGE_CHILDREN.eq(&id.value) {
            if let Some(props_arg) = props_arg {
              if matches!(expr.obj.as_ref(), Expr::Ident(id) if id.sym.eq(props_arg)) {
                SlotName::Default
              } else {
                SlotName::None
              }
            } else {
              SlotName::None
            }
          } else if id.value.starts_with("slot:") {
            let name = id.value[5..].into();
            if let Some(props_arg) = props_arg {
              if matches!(expr.obj.as_ref(), Expr::Ident(id) if id.sym.eq(props_arg)) {
                SlotName::Named(name)
              } else {
                SlotName::Expr(Box::new(Expr::Member(expr.clone())))
              }
            } else {
              SlotName::Expr(Box::new(Expr::Member(expr.clone())))
            }
          } else {
            SlotName::None
          }
        }
        _ => SlotName::None,
      },
      _ => SlotName::None,
    },
    _ => SlotName::None,
  }
}

#[inline]
fn get_slot_name_from_callee(callee: &Expr, props_arg: &Option<Atom>) -> SlotName {
  match callee {
    Expr::Member(e) => get_slot_name_from_member_expr(e, props_arg),
    Expr::OptChain(oc) => {
      if let OptChainBase::Member(m) = oc.base.as_ref() {
        get_slot_name_from_member_expr(m, props_arg)
      } else {
        SlotName::None
      }
    }
    _ => SlotName::None,
  }
}

/// 将插槽名称（SlotName）转成实际的插槽渲染函数的函数体（Callee）比如：
/// props.children ==> host[SLOTS][DEFAULT_SLOT]
/// props['slot:X'] ==> host[SLOTS].X
/// a['slot:X'] => a['slot:X']
pub fn slot_name_to_callee_expr(slot_name: SlotName) -> Box<Expr> {
  match slot_name {
    SlotName::Expr(e) => e,
    SlotName::Default => ast_create_expr_member(
      ast_create_expr_member(
        ast_create_expr_ident(JINGE_ROOT_HOST_IDENT.clone()),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(JINGE_IMPORT_SLOTS.local()),
        }),
      ),
      MemberProp::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: ast_create_expr_ident(JINGE_IMPORT_DEFAULT_SLOT.local()),
      }),
    ),
    SlotName::Named(n) => ast_create_expr_member(
      ast_create_expr_member(
        ast_create_expr_ident(JINGE_ROOT_HOST_IDENT.clone()),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(JINGE_IMPORT_SLOTS.local()),
        }),
      ),
      MemberProp::Ident(IdentName::from(n)),
    ),
    _ => panic!(),
  }
}

#[inline]
fn exprorspread_vec_to_expr(mut exprorspread_vec: Vec<ExprOrSpread>) -> Box<Expr> {
  if exprorspread_vec.is_empty() {
    Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
  } else if exprorspread_vec.len() == 1 {
    let e = exprorspread_vec.pop().unwrap();
    if e.spread.is_some() {
      e.expr
    } else {
      Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![Some(e)],
      }))
    }
  } else {
    Box::new(Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: exprorspread_vec.into_iter().map(|e| Some(e)).collect(),
    }))
  }
}

struct SlotVm {
  pub const_props: Vec<(PropName, Box<Expr>)>,
  pub watch_props: Vec<(PropName, ExprParseResult)>,
  pub spread_prop: Option<Ident>,
}

fn parse_slot_arg_prop(vm: &mut SlotVm, prop: &Prop) {
  let kv = match prop {
    Prop::Shorthand(k) => {
      // 形如 { someVar } 这样的简写，等价于 { someVar: someVar }
      let e = Box::new(Expr::Ident(k.clone()));
      let r = ExprVisitor::new().parse(&e);
      match r {
        ExprParseResult::None => (), // 这种简写不可能是 ExprParseResult::None
        _ => vm.watch_props.push((PropName::Ident(k.clone().into()), r)),
      }
      return;
    }
    Prop::KeyValue(kv) => kv,
    _ => {
      emit_error(prop.span(), "Slot 渲染参数必须是 key-value 类型的 Object。");
      return;
    }
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
      if match &kv.key {
        PropName::Str(s) => {
          if s.value.starts_with("on:") {
            true
          } else {
            false
          }
        }
        PropName::Ident(s) => {
          if s.sym.starts_with("on:") {
            true
          } else {
            false
          }
        }
        _ => false,
      } {
        vm.const_props.push((kv.key.clone(), kv.value.clone()));
      } else {
        emit_error(
          kv.value.span(),
          "不支持函数作为插槽的传递数据。如果是要传递事件函数，请使用 on: 打头的事件名。",
        );
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
        parse_slot_arg_prop(&mut vm, prop.as_ref());
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
  fn transform_slot_to_render_fn(
    &mut self,
    slot_name: SlotName,
    slot_args: Option<&Vec<ExprOrSpread>>,
  ) -> Box<Expr> {
    let mut stmts = vec![];

    let slot_vm_id =
      slot_args.and_then(|slot_args| self.transform_slot_args(slot_args, &mut stmts));

    let host_ident = self.create_host_ident();

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_NEW_COM_DEFAULT_SLOT.local()),
        vec![ast_create_arg_expr(ast_create_expr_member(
          ast_create_expr_ident(host_ident.clone()),
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
          }),
        ))],
      ),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), host_ident),
    }));

    let mut args = vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      ast_create_arg_expr(slot_name_to_callee_expr(slot_name)),
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
  fn transform_slot(&mut self, slot_name: SlotName, slot_args: Option<&Vec<ExprOrSpread>>) {
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

        let host_ident = self.create_host_ident();
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_render(set_fn, watch_expr, host_ident),
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
    let slot_name = get_slot_name_from_member_expr(expr, &self.props_arg);
    if matches!(slot_name, SlotName::None) {
      return false;
    }
    self.transform_slot(slot_name, slot_args);
    true
  }
  pub fn parse_slot_call_expr(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    let slot_name = get_slot_name_from_callee(callee, &self.props_arg);
    if matches!(slot_name, SlotName::None) {
      return false;
    }
    self.transform_slot(slot_name, Some(args));
    true
  }

  fn get_bin_expr_slot_name(&self, expr: &Expr) -> SlotName {
    let Expr::Member(mem) = expr else {
      return SlotName::None;
    };

    get_slot_name_from_member_expr(mem, &self.props_arg)
  }
  fn parse_expr_to_render_fn(&mut self, expr: &Expr) -> Vec<ExprOrSpread> {
    self.push_context(self.context.parent);
    self.visit_expr(expr);
    let mut context = self.pop_context();
    let Some(s) = context.slots.pop() else {
      return vec![];
    };
    s.expressions
  }
  pub fn parse_cond_slot(&mut self, expr: &CondExpr) -> bool {
    let slot_name = self.get_bin_expr_slot_name(&expr.test);
    if matches!(slot_name, SlotName::None) {
      return false;
    }
    let conds_render_fn = self.parse_expr_to_render_fn(&expr.cons);
    let alt_render_fn = self.parse_expr_to_render_fn(&expr.alt);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name_to_callee_expr(slot_name),
      cons: exprorspread_vec_to_expr(conds_render_fn),
      alt: exprorspread_vec_to_expr(alt_render_fn),
    })));
    true
  }

  pub fn parse_logic_and_slot(&mut self, expr: &BinExpr) -> bool {
    let slot_name = self.get_bin_expr_slot_name(&expr.left);
    if matches!(slot_name, SlotName::None) {
      return false;
    }
    let render_fn = self.parse_expr_to_render_fn(&expr.right);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name_to_callee_expr(slot_name),
      cons: exprorspread_vec_to_expr(render_fn),
      alt: Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![],
      })),
    })));
    true
  }

  /// 解析由通过逻辑或指定默认插槽的表达式。
  /// 例如 `<>{props.children || <div>default</div>}</>`
  /// 本质上和 `<>{props.children ?? <div>default</div>}` 完全一致，因为插槽表达式如果判定为[假]，则只可能是 undefined。
  pub fn parse_logic_or_slot(&mut self, expr: &BinExpr) -> bool {
    self.parse_nullish_coalescing_slot(expr)
  }

  /// 解析通过 ?? 指定默认插槽的表达式。例如：
  /// ```
  /// <>{props.children ?? <div>default</div>}</>
  /// <>{props['slot:xx'] ?? <div>default</div>}</>
  /// <>{props.children?.() ?? <div>default</div>}</>
  /// ```
  /// 其中最后一种类型比较特殊，`??` 运算符本来应该是判定插槽函数执行后的结果，
  /// 但因为插槽函数是特殊的写法，并不会真执行函数也没有返回值，所以实际仍然是判定函数本身是否存在，
  /// 即判定插槽是否存在，这种情况本质等价于：
  /// ```
  /// <>{props.children ? props.children() : <div>default</div>}</>
  /// ```
  pub fn parse_nullish_coalescing_slot(&mut self, expr: &BinExpr) -> bool {
    let mut slot_name = self.get_bin_expr_slot_name(&expr.left);
    let render_fn = if matches!(slot_name, SlotName::None) {
      // 如果 ?? 左边不是简单的 member 表达式插槽，即不是 `props.children ?? 'default'` 这种表达式。
      // 则继续看是否是 `props.children?.() ?? 'default'` 这样的表达式。
      match expr.left.as_ref() {
        Expr::OptChain(opt) => match opt.base.as_ref() {
          OptChainBase::Call(c) => {
            slot_name = get_slot_name_from_callee(&c.callee, &self.props_arg);
            if matches!(slot_name, SlotName::None) {
              return false;
            }
            self.transform_slot_to_render_fn(slot_name.clone(), Some(&c.args))
          }
          _ => return false,
        },
        _ => {
          return false;
        }
      }
    } else {
      self.transform_slot_to_render_fn(slot_name.clone(), None)
    };

    let default_slot = self.parse_expr_to_render_fn(&expr.right);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name_to_callee_expr(slot_name),
      cons: render_fn,
      alt: exprorspread_vec_to_expr(default_slot),
    })));
    true
  }
}

use swc_common::{DUMMY_SP, Spanned, SyntaxContext};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    ArrayLit, AssignExpr, AssignOp, AssignTarget, BinExpr, BlockStmt, BlockStmtOrExpr, Callee,
    ComputedPropName, CondExpr, Expr, ExprOrSpread, ExprStmt, Ident, KeyValueProp, Lit, MemberExpr,
    MemberProp, NewExpr, Null, ObjectLit, OptChainBase, OptChainExpr, Prop, PropName, PropOrSpread,
    ReturnStmt, SimpleAssignTarget, Stmt,
  },
};
use swc_ecma_visit::Visit;

use crate::{
  ast::*,
  common::{JINGE_IMPORT_COMPONENT_HOST, JINGE_SLOT_DEFAULT},
  parser::{
    JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_V_IDENT, expr::ExprVisitor, tpl::tpl_watch_and_render,
  },
};

use super::{
  JINGE_CHILDREN, JINGE_EL_IDENT, JINGE_IMPORT_CONTEXT, JINGE_IMPORT_RENDER_SLOT, TemplateParser,
  emit_error, expr::ExprParseResult, tpl::tpl_push_el_code,
};

#[inline]
fn get_default_slot_name(props_arg: &Atom) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: ast_create_expr_ident(props_arg.clone().into()),
    prop: MemberProp::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: ast_create_expr_lit_str(JINGE_SLOT_DEFAULT.clone().into()),
    }),
  }
}

enum SlotNameType {
  Default,
  Expr,
  None,
}

fn get_slot_name_type_from_member_epxr(
  expr: &MemberExpr,
  props_arg: &Option<Atom>,
) -> SlotNameType {
  match &expr.prop {
    MemberProp::Ident(id) => {
      let Some(props_arg) = props_arg else {
        return SlotNameType::None;
      };
      if JINGE_CHILDREN.eq(&id.sym)
        && matches!(expr.obj.as_ref(), Expr::Ident(id) if id.sym.eq(props_arg))
      {
        return SlotNameType::Default;
      }
    }
    MemberProp::Computed(e) => match e.expr.as_ref() {
      Expr::Lit(id) => match id {
        Lit::Str(id) => {
          if id.value.as_atom().map_or(false, |v| JINGE_CHILDREN.eq(v)) {
            if let Some(props_arg) = props_arg {
              if matches!(expr.obj.as_ref(), Expr::Ident(id) if id.sym.eq(props_arg)) {
                return SlotNameType::Default;
              }
            }
          } else if id.value.starts_with("slot:") {
            return SlotNameType::Expr;
          }
        }
        _ => (),
      },
      _ => (),
    },
    _ => (),
  }
  SlotNameType::None
}

fn get_slot_name_type_from_optchain_expr<'a>(
  expr: &'a OptChainExpr,
  props_arg: &Option<Atom>,
) -> (SlotNameType, Option<&'a Vec<ExprOrSpread>>) {
  match expr.base.as_ref() {
    OptChainBase::Member(m) => (get_slot_name_type_from_member_epxr(m, props_arg), None),
    OptChainBase::Call(c) => match c.callee.as_ref() {
      Expr::Member(mem) => (
        get_slot_name_type_from_member_epxr(mem, props_arg),
        Some(&c.args),
      ),
      Expr::OptChain(opt) => match opt.base.as_ref() {
        OptChainBase::Member(m) => (
          get_slot_name_type_from_member_epxr(m, props_arg),
          Some(&c.args),
        ),
        _ => (SlotNameType::None, None),
      },
      _ => (SlotNameType::None, None),
    },
  }
}

#[inline]
pub fn get_slot_name_from_member_expr(
  expr: &MemberExpr,
  props_arg: &Option<Atom>,
) -> Option<Box<Expr>> {
  match get_slot_name_type_from_member_epxr(expr, props_arg) {
    SlotNameType::None => None,
    SlotNameType::Default => Some(Box::new(Expr::Member(get_default_slot_name(
      props_arg.as_ref().unwrap(),
    )))),
    SlotNameType::Expr => Some(Box::new(Expr::Member(expr.clone()))),
  }
}

pub fn get_slot_name_from_optchain_expr<'a>(
  expr: &'a OptChainExpr,
  props_arg: &Option<Atom>,
) -> Option<(Box<Expr>, Option<&'a Vec<ExprOrSpread>>)> {
  match get_slot_name_type_from_optchain_expr(expr, props_arg) {
    (SlotNameType::None, _) => None,
    (SlotNameType::Default, args) => Some((
      Box::new(Expr::OptChain(OptChainExpr {
        span: DUMMY_SP,
        optional: true,
        base: Box::new(OptChainBase::Member(get_default_slot_name(
          props_arg.as_ref().unwrap(),
        ))),
      })),
      args,
    )),
    (SlotNameType::Expr, args) => match expr.base.as_ref() {
      OptChainBase::Call(call) => Some((call.callee.clone(), args)),
      OptChainBase::Member(mem) => Some((Box::new(Expr::Member(mem.clone())), args)),
    },
  }
}

pub fn get_slot_name_from_callee(callee: &Expr, props_arg: &Option<Atom>) -> Option<Box<Expr>> {
  match callee {
    Expr::Member(e) => get_slot_name_from_member_expr(e, props_arg),
    Expr::OptChain(oc) => {
      if let OptChainBase::Member(m) = oc.base.as_ref() {
        get_slot_name_from_member_expr(m, props_arg)
      } else {
        None
      }
    }
    _ => None,
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
        ExprParseResult::None => vm.const_props.push((PropName::Ident(k.clone().into()), e)), // 这种简写不可能是 ExprParseResult::None
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

pub fn get_bin_expr_slot_name<'a>(
  expr: &'a Expr,
  props_arg: &Option<Atom>,
) -> Option<(Box<Expr>, Option<&'a Vec<ExprOrSpread>>)> {
  match expr {
    Expr::Member(mem) => {
      get_slot_name_from_member_expr(mem, props_arg).map(|slot_name| (slot_name, None))
    }
    Expr::OptChain(opt) => get_slot_name_from_optchain_expr(opt, props_arg),
    Expr::Call(call) => match &call.callee {
      Callee::Expr(callee) => get_slot_name_from_callee(callee.as_ref(), props_arg)
        .map(|slot_name| (slot_name, Some(&call.args))),
      _ => None,
    },
    _ => None,
  }
}
impl TemplateParser {
  fn transform_slot_to_render_fn(
    &mut self,
    slot_name: Box<Expr>,
    slot_args: Option<&Vec<ExprOrSpread>>,
  ) -> Box<Expr> {
    let mut stmts = vec![];

    let slot_vm_id =
      slot_args.and_then(|slot_args| self.transform_slot_args(slot_args, &mut stmts));
    let host_ident = self.create_host_ident();

    stmts.push(ast_create_stmt_decl_const(
      JINGE_EL_IDENT.clone(),
      Box::new(Expr::New(NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
        callee: ast_create_expr_ident(JINGE_IMPORT_COMPONENT_HOST.local()),
        args: Some(vec![ast_create_arg_expr(ast_create_expr_member(
          ast_create_expr_ident(host_ident.clone()),
          MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: ast_create_expr_ident(JINGE_IMPORT_CONTEXT.local()),
          }),
        ))]),
        type_args: None,
      })),
    ));
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(self.context.is_parent_component(), host_ident),
    }));

    let mut args = vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      ast_create_arg_expr(slot_name),
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
  fn transform_slot(&mut self, slot_name: Box<Expr>, slot_args: Option<&Vec<ExprOrSpread>>) {
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
    // slot_args: Option<&Vec<ExprOrSpread>>,
  ) -> bool {
    if let Some(slot_name) = get_slot_name_from_member_expr(expr, &None) {
      self.transform_slot(slot_name, None);
      true
    } else {
      false
    }
  }
  pub fn parse_slot_call_expr(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    if let Some(slot_name) = get_slot_name_from_callee(callee, &self.props_arg) {
      self.transform_slot(slot_name, Some(args));
      true
    } else {
      return false;
    }
  }
  pub fn parse_slot_optchain_expr(&mut self, expr: &OptChainExpr) -> bool {
    if let Some((slot_name, slot_args)) = get_slot_name_from_optchain_expr(expr, &self.props_arg) {
      self.transform_slot(slot_name, slot_args);
      true
    } else {
      false
    }
  }

  fn parse_expr_to_render_fn(&mut self, expr: &Expr) -> Vec<ExprOrSpread> {
    self.push_context_without_inc_deep(self.context.parent);
    self.visit_expr(expr);
    let mut context = self.pop_context_without_dec_deep();
    let Some(s) = context.slots.pop() else {
      return vec![];
    };
    s.expressions
  }
  pub fn parse_cond_slot(&mut self, expr: &CondExpr) -> bool {
    let Some((slot_name, slot_args)) = get_bin_expr_slot_name(&expr.test, &self.props_arg) else {
      return false;
    };
    if slot_args.is_some() {
      emit_error(expr.span(), "? : 表达式中的插槽错误。");
    }
    let conds_render_fn = self.parse_expr_to_render_fn(&expr.cons);
    let alt_render_fn = self.parse_expr_to_render_fn(&expr.alt);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name,
      cons: exprorspread_vec_to_expr(conds_render_fn),
      alt: exprorspread_vec_to_expr(alt_render_fn),
    })));
    true
  }

  pub fn parse_logic_and_slot(&mut self, expr: &BinExpr) -> bool {
    let Some((slot_name, slot_args)) = get_bin_expr_slot_name(&expr.left, &self.props_arg) else {
      return false;
    };
    if slot_args.is_some() {
      emit_error(expr.span(), "&& 表达式中的插槽错误。");
    }
    let render_fn = self.parse_expr_to_render_fn(&expr.right);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name,
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
    let Some((slot_name, slot_args)) = get_bin_expr_slot_name(&expr.left, &self.props_arg) else {
      return false;
    };
    // println!("OOO {:#?}", slot_name);
    let render_fn = self.transform_slot_to_render_fn(slot_name.clone(), slot_args);
    let default_slot = self.parse_expr_to_render_fn(&expr.right);
    self.push_expression_with_spread(Box::new(Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: slot_name,
      cons: render_fn,
      alt: exprorspread_vec_to_expr(default_slot),
    })));
    true
  }
}

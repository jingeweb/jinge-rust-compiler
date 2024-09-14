use swc_common::Spanned;
use swc_core::{atoms::Atom, common::DUMMY_SP, ecma::ast::*};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::{
  ast::{ast_create_expr_ident, ast_create_expr_member},
  parser::JINGE_LOOP_EACH_DATA,
};

use super::{
  emit_error, map_key::KeyFnFindVisitor, TemplateParser, JINGE_IMPORT_FOR, JINGE_LOOP,
  JINGE_LOOP_EACH_IDENTS, JINGE_LOOP_EACH_INDEX, JINGE_LOOP_KEY_FN, JINGE_MAP,
};

/// map 循环转换成 <For> 组件时，需要把 map 函数的参数，转成 <For> 组件的 Slot 函数的参数。
/// 比如：
/// ```tsx
/// render() {
///   return <div>{this.boys.map((boy, idx) => <span key={boy.id}>{idx} - {boy.name}</span>)}</div>
/// }
/// ```
/// 需要被转换成：
/// ```tsx
/// render() {
///   return <div><For key={(each) => each.data.id}>
///   {
///     (each) => <span>{each.index} - {each.data.name}</span>
///   }
///    </For></div>
/// }
/// ```
/// 也就是说 map 第一个参数需要全部替换为 `each.data`，第二个参数全部替换为 `each.index`.
///
/// 实际情况会更复杂，map 函数会有嵌套的情况，因此 `each` 需要每一层有不同的名称。
///
/// 此外如果 map 函数的 body 里有嵌套的 map 或其它 slot 函数，且函数的参数正好重名，
/// 各种语言内部 scope 定义的参数名 override 上一层 scope 时都将失去对上层 scope 定义的该参数。
/// 因此遇到这种情况则不再需要进一步深入递归替换。
struct ReplaceVisitor {
  arg_data: Option<Atom>,
  arg_index: Option<Atom>,
  slot_vm_name: Atom,
  stack: Vec<(bool, bool)>,
}
#[inline]
fn pat_to_atom(p: Option<&Pat>) -> Option<Atom> {
  p.and_then(|p| {
    if let Pat::Ident(id) = p {
      Some(id.sym.clone())
    } else {
      None
    }
  })
}

impl ReplaceVisitor {
  #[inline]
  fn new(arg_data: Option<Atom>, arg_index: Option<Atom>, slot_vm_name: Atom) -> Self {
    let overrided = (arg_data.is_none(), arg_index.is_none());
    Self {
      arg_data,
      arg_index,
      slot_vm_name,
      stack: vec![overrided],
    }
  }
  #[inline]
  fn all_params_overrided(&self) -> bool {
    let overrided = self.stack.last().unwrap();
    overrided.0 && overrided.1
  }
  #[inline]
  fn check_p(&mut self, par: &Pat) -> bool {
    let Pat::Ident(par) = par else {
      return false;
    };
    let overrided = self.stack.last_mut().unwrap();
    if !overrided.0 && matches!(&self.arg_data, Some(arg_data) if arg_data.eq(&par.sym)) {
      overrided.0 = true;
    }
    if !overrided.1 && matches!(&self.arg_index, Some(arg_index) if arg_index.eq(&par.sym)) {
      overrided.1 = true;
    }
    overrided.0 && overrided.1
  }
  /// 检查参数是否已经全部被覆盖。如果根参数 v0, v1 在嵌套函数的参数中被覆盖，则这个函数内部的同名参数都不再需要被替换成 slot 参数。
  fn check_params_override(&mut self, params: &Vec<Param>) -> bool {
    for par in params.iter() {
      if self.check_p(&par.pat) {
        return true;
      }
    }
    false
  }
  fn check_params_override_2(&mut self, params: &Vec<Pat>) -> bool {
    for par in params.iter() {
      if self.check_p(par) {
        return true;
      }
    }
    false
  }
}
impl VisitMut for ReplaceVisitor {
  fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
    self.stack.push(self.stack.last().unwrap().clone());
    if !self.check_params_override(&node.function.params) {
      if let Some(body) = &mut node.function.body {
        body.visit_mut_children_with(self);
      }
    }
    self.stack.pop();
  }
  fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
    self.stack.push(self.stack.last().unwrap().clone());
    if !self.check_params_override(&node.function.params) {
      if let Some(body) = &mut node.function.body {
        body.visit_mut_children_with(self);
      }
    }
    self.stack.pop();
  }
  fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
    self.stack.push(self.stack.last().unwrap().clone());
    if !self.check_params_override_2(&node.params) {
      node.body.as_mut().visit_mut_children_with(self);
    }
    self.stack.pop();
  }
  fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
    match node.obj.as_ref() {
      Expr::Ident(id) => {
        let overrided: &(bool, bool) = self.stack.last().unwrap();
        if !overrided.0 && matches!(self.arg_data, Some(ref a) if a.eq(&id.sym)) {
          // println!("replace mem expr");
          node.obj = ast_create_expr_member(
            ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_DATA.clone())),
          )
        } else if !overrided.1 && matches!(self.arg_index, Some(ref a) if a.eq(&id.sym)) {
          node.obj = ast_create_expr_member(
            ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_INDEX.clone())),
          )
        }
      }
      _ => node.visit_mut_children_with(self),
    }
  }
  fn visit_mut_jsx_expr(&mut self, node: &mut JSXExpr) {
    if let JSXExpr::Expr(e) = node {
      if let Expr::Ident(id) = e.as_ref() {
        let overrided: &(bool, bool) = self.stack.last().unwrap();
        if !overrided.0 && matches!(self.arg_data, Some(ref a) if a.eq(&id.sym)) {
          // println!("replace ident expr");
          *e = ast_create_expr_member(
            ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_DATA.clone())),
          );
        } else if !overrided.1 && matches!(self.arg_index, Some(ref a) if a.eq(&id.sym)) {
          *e = ast_create_expr_member(
            ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_INDEX.clone())),
          )
        }
        return; // 重要！Ident 类型不再需要后续的 visit_mut_children_with
      }
    }
    node.visit_mut_children_with(self);
  }
}

fn gen_for_component(looop: &Box<Expr>, key: Option<Box<Expr>>, func: ArrowExpr) -> JSXElement {
  let mut attrs = vec![JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: looop.span(),
    name: JSXAttrName::Ident(IdentName::from(JINGE_LOOP.clone())),
    value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: looop.span(),
      expr: JSXExpr::Expr(looop.clone()),
    })),
  })];
  if let Some(key) = key {
    attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::Ident(IdentName::from(JINGE_LOOP_KEY_FN.clone())),
      value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        span: key.span(),
        expr: JSXExpr::Expr(key),
      })),
    }))
  };
  JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      name: JSXElementName::Ident(JINGE_IMPORT_FOR.local()),
      span: DUMMY_SP,
      attrs,
      self_closing: false,
      type_args: None,
    },
    children: vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(Expr::Arrow(func))),
    })],
    closing: Some(JSXClosingElement {
      name: JSXElementName::Ident(JINGE_IMPORT_FOR.local()),
      span: DUMMY_SP,
    }),
  }
}
impl TemplateParser {
  /// 如果表达式是 xx.map() 调用，且参数只有一个，参数是箭头函数，则转换为 <For> 组件。
  pub fn parse_map_fn(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    if args.len() != 1 {
      return false;
    }
    let arg0 = &args[0];
    if arg0.spread.is_some() {
      return false;
    };

    let looop = match callee {
      Expr::Member(m) => {
        if matches!(&m.prop, MemberProp::Ident(fname) if JINGE_MAP.eq(&fname.sym)) {
          &m.obj
        } else {
          return false;
        }
      }
      Expr::OptChain(oc) => {
        if let OptChainBase::Member(m) = oc.base.as_ref() {
          if matches!(&m.prop, MemberProp::Ident(fname) if JINGE_MAP.eq(&fname.sym)) {
            &m.obj
          } else {
            return false;
          }
        } else {
          return false;
        }
      }
      _ => {
        return false;
      }
    };

    let mut func = match arg0.expr.as_ref() {
      Expr::Arrow(e) => e.clone(),
      Expr::Fn(func) => {
        emit_error(func.span(), "告警：map 函数的参数请使用箭头函数！");
        return false;
      }
      _ => {
        return false;
      }
    };

    // 一般情况下，map 嵌套不会太多。小于 JINGE_LOOP_EACH_IDENTS.len() 层直接用预置好的 Atom，否则才用 format! 动态拼接。
    let slot_vm_name = JINGE_LOOP_EACH_IDENTS
      .get(self.map_loop_level)
      .map(|v| v.clone())
      .unwrap_or_else(|| Atom::from(format!("each$jg${}", self.map_loop_level)));
    let arg_data = pat_to_atom(func.params.get(0));
    let arg_index = pat_to_atom(func.params.get(1));
    let mut replace_visitor =
      ReplaceVisitor::new(arg_data.clone(), arg_index.clone(), slot_vm_name.clone());
    func.params = vec![Pat::Ident(BindingIdent::from(slot_vm_name.clone()))];

    let find_key_visitor = KeyFnFindVisitor {
      arg_data,
      arg_index,
      // slot_vm_name,
    };
    let key_fn = find_key_visitor.get_key_fn(&func);

    // replace_visitor 必须在 find_key_visitor 之后执行，因为 replace_visitor 也会把 key 属性值里的表达式替换。
    if !replace_visitor.all_params_overrided() {
      func.body.visit_mut_children_with(&mut replace_visitor);
    }

    let for_component = gen_for_component(looop, key_fn, func);
    let tn = Ident::from(JINGE_IMPORT_FOR.local());

    self.map_loop_level += 1;
    self.parse_component_element(&tn, &for_component);
    self.map_loop_level -= 1;

    true
  }
}

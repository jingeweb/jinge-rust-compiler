use swc_common::Spanned;
use swc_core::{atoms::Atom, common::DUMMY_SP, ecma::ast::*};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::{
  ast::{ast_create_expr_ident, ast_create_expr_lit_str},
  common::{JINGE_IMPORT_KEY_DATA, JINGE_IMPORT_KEY_INDEX, JINGE_KEY, emit_warn},
  parser::JINGE_LOOP_EACH_DATA,
};

use super::{
  JINGE_IMPORT_FOR, JINGE_LOOP, JINGE_LOOP_EACH_IDENTS, JINGE_LOOP_EACH_INDEX, JINGE_MAP,
  TemplateParser, emit_error,
  map_key::{MapKey, MapKeyFindVisitor},
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
/// SWC resolver 的绑定 Id 区分所有词法遮蔽；嵌套函数仍可引用外层参数，
/// 因此继续遍历，只替换绑定身份相同的引用。
struct ReplaceVisitor {
  arg_data: Option<Id>,
  arg_index: Option<Id>,
  slot_vm_name: Atom,
}
#[inline]
fn pat_to_id(p: Option<&Pat>) -> Option<Id> {
  p.and_then(|p| {
    if let Pat::Ident(id) = p {
      Some(id.id.to_id())
    } else {
      None
    }
  })
}

impl ReplaceVisitor {
  #[inline]
  fn new(arg_data: Option<Id>, arg_index: Option<Id>, slot_vm_name: Atom) -> Self {
    Self {
      arg_data,
      arg_index,
      slot_vm_name,
    }
  }
  fn has_params(&self) -> bool {
    self.arg_data.is_some() || self.arg_index.is_some()
  }
}
impl VisitMut for ReplaceVisitor {
  fn visit_mut_expr(&mut self, e: &mut Expr) {
    match e {
      Expr::Ident(id) => {
        if matches!(self.arg_data, Some(ref a) if a.eq(&id.to_id())) {
          *e = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            prop: MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_DATA.clone())),
          });
        } else if matches!(self.arg_index, Some(ref a) if a.eq(&id.to_id())) {
          *e = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
            prop: MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_INDEX.clone())),
          });
        }
      }
      Expr::Object(obj) => {
        // 如果是 `{ index }` 这种写法，也要转成 `{ index: slot_vm_name[JINGE_LOOP_EACH_INDEX] }`
        for prop in obj.props.iter_mut() {
          match prop {
            PropOrSpread::Spread(x) => {
              x.visit_mut_children_with(self);
            }
            PropOrSpread::Prop(p) => match p.as_mut() {
              Prop::Shorthand(s) => {
                if matches!(self.arg_data, Some(ref a) if a.eq(&s.to_id())) {
                  **p = Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(IdentName::from(s.sym.clone())),
                    value: Box::new(Expr::Member(MemberExpr {
                      span: DUMMY_SP,
                      obj: ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
                      prop: MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_DATA.clone())),
                    })),
                  });
                } else if matches!(self.arg_index, Some(ref a) if a.eq(&s.to_id())) {
                  **p = Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(IdentName::from(s.sym.clone())),
                    value: Box::new(Expr::Member(MemberExpr {
                      span: DUMMY_SP,
                      obj: ast_create_expr_ident(Ident::from(self.slot_vm_name.clone())),
                      prop: MemberProp::Ident(IdentName::from(JINGE_LOOP_EACH_INDEX.clone())),
                    })),
                  })
                }
              }
              _ => p.visit_mut_children_with(self),
            },
          }
        }
      }
      _ => e.visit_mut_children_with(self),
    }
  }
}

fn gen_for_component(looop: &Expr, key: MapKey, func: ArrowExpr) -> JSXElement {
  let mut attrs = vec![JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: looop.span(),
    name: JSXAttrName::Ident(IdentName::from(JINGE_LOOP.clone())),
    value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: looop.span(),
      expr: JSXExpr::Expr(Box::new(looop.clone())),
    })),
  })];
  if let Some(key_attr_value) = match key {
    MapKey::Data => Some(ast_create_expr_ident(JINGE_IMPORT_KEY_DATA.local())),
    MapKey::Index => Some(ast_create_expr_ident(JINGE_IMPORT_KEY_INDEX.local())),
    MapKey::Prop(path) => Some(ast_create_expr_lit_str(path.into())),
    MapKey::Expr(expr) => Some(expr),
    _ => None,
  } {
    attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::Ident(IdentName::from(JINGE_KEY.clone())),
      value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(key_attr_value),
      })),
    }));
  }

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
  pub fn parse_map_fn(&mut self, callee: &Expr, args: &[ExprOrSpread]) -> bool {
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

    if let Expr::Member(member) = callee {
      crate::scope::warn_member(member);
    }
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

    if func.params.len() > 2 || func.params.iter().any(|p| !matches!(p, Pat::Ident(_))) {
      emit_error(
        func.span,
        "map 模板参数仅支持 data、index 两个普通标识符，不支持解构、默认、rest 或第三个参数",
      );
      return true;
    }
    // 一般情况下，map 嵌套不会太多。小于 JINGE_LOOP_EACH_IDENTS.len() 层直接用预置好的 Atom，否则才用 format! 动态拼接。
    let slot_vm_name = JINGE_LOOP_EACH_IDENTS
      .get(self.map_loop_level)
      .cloned()
      .unwrap_or_else(|| Atom::from(format!("each$jg${}", self.map_loop_level)));
    let arg_data = pat_to_id(func.params.first());
    let arg_index = pat_to_id(func.params.get(1));
    for param in [&arg_data, &arg_index].into_iter().flatten() {
      if crate::scope::binding_is_written(&func.body, param) {
        emit_error(
          func.span,
          "map 模板参数不支持重新赋值；请修改对象成员或使用独立的 vm 状态",
        );
        return true;
      }
    }
    let mut replace_visitor =
      ReplaceVisitor::new(arg_data.clone(), arg_index.clone(), slot_vm_name.clone());
    func.params = vec![Pat::Ident(BindingIdent::from(slot_vm_name.clone()))];

    let find_key_visitor = MapKeyFindVisitor {
      arg_data,
      arg_index,
      // slot_vm_name,
    };
    let map_key = find_key_visitor.get_key(&mut func);
    if map_key.is_none() {
      emit_warn(
        func.span(),
        "map 函数转换为 <For> 组件时缺失 key，建议始终指定 key 用于提升性能",
      );
    }

    // replace_visitor 必须在 find_key_visitor 之后执行，因为 replace_visitor 也会把 key 属性值里的表达式替换。
    if replace_visitor.has_params() {
      func.body.visit_mut_children_with(&mut replace_visitor);
    }

    let for_component = gen_for_component(looop, map_key, func);
    let tn = JINGE_IMPORT_FOR.local();

    self.map_loop_level += 1;
    self.parse_component_element(&tn, &for_component);
    self.map_loop_level -= 1;

    true
  }
}

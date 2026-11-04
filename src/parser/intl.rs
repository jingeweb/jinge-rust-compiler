use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha512};
use swc_common::{DUMMY_SP, Spanned};
use swc_core::{atoms::Wtf8Atom, ecma::ast::*};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

use super::{
  IntlType, JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_KEY, JINGE_V_IDENT, TemplateParser,
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
  ast_create_expr_member, ast_create_stmt_decl_const, emit_error,
  expr::{ExprParseResult, ExprVisitor},
  tpl_render_intl_normal_text, tpl_render_intl_text, tpl_watch_and_render,
};

/// 计算文本的 hash。
/// 需要和 /ts/intl/extract/helper.rs 中使用算法一致，当前统一为 sha512().toBase64().slice(0,6)。
/// 如果修改两处都要变更。
pub fn calc_intl_key(message: &str, filename: Option<&str>) -> String {
  let mut h = Sha512::new();
  h.update(message);
  if let Some(filename) = filename {
    h.update(filename)
  }
  let h = h.finalize();
  // println!("hash len: {:?}", h.len());
  let mut enc_buf = [0u8; 128];

  let x = Base64::encode(&h, &mut enc_buf).unwrap();
  x[0..6].to_string()
}

struct IntlParams {
  pub is_rich_text: bool,
  pub const_props: Vec<(PropName, Box<Expr>)>,
  pub watch_props: Vec<(PropName, ExprParseResult)>,
}

pub fn extract_t(args: &[ExprOrSpread]) -> Option<(Wtf8Atom, &Wtf8Atom, Option<&ObjectLit>)> {
  let default_text = args.first()?;
  if default_text.spread.is_some() {
    return None;
  }
  let default_text_span = default_text.span();
  let Expr::Lit(Lit::Str(default_text)) = default_text.expr.as_ref() else {
    emit_error(
      default_text_span,
      "t 函数的第一个参数必须是字符串常量，代表默认文本",
    );
    return None;
  };

  let mut key = None;
  // let mut isolate = false;
  let options_arg = args.get(2);
  if let Some(options) = options_arg {
    if options.spread.is_some() {
      emit_error(options.span(), "t 函数的 options 参数不支持 ... 解构写法");
    } else if let Expr::Object(opts) = options.expr.as_ref() {
      for prop in opts.props.iter() {
        if let PropOrSpread::Prop(prop) = prop
          && let Prop::KeyValue(kv) = prop.as_ref()
          && let PropName::Ident(id) = &kv.key
          && JINGE_KEY.eq(&id.sym)
        {
          if let Expr::Lit(Lit::Str(v)) = kv.value.as_ref() {
            key = v
              .value
              .as_str()
              .map(str::trim)
              .filter(|key| !key.is_empty())
              .map(Wtf8Atom::from);
          }
          // 找到 key 就退出循环。
          break;
        }
      }
    }
  }

  let params_arg = args.get(1).and_then(|p| {
    if p.spread.is_some() {
      emit_error(p.span(), "t 函数的 params 参数不支持 ... 解构写法");
      None
    } else if let Expr::Object(expr) = p.expr.as_ref() {
      Some(expr)
    } else {
      emit_error(options_arg.span(), "t 函数的 params 参数必须是 object 类型");
      None
    }
  });

  if key.is_none() {
    let Some(v) = default_text.value.as_str() else {
      emit_error(default_text_span, "t 函数的默认文本解析异常");
      return None;
    };
    key = Some(calc_intl_key(v, None).into());
  }

  Some((key.unwrap(), &default_text.value, params_arg))
}
impl TemplateParser {
  /// Compile a rich renderer where it was declared, retaining its lexical scope.
  /// Its one public argument is translated content (a static slot), not component props.
  fn parse_intl_renderer(&self, expr: &Expr) -> Option<Box<Expr>> {
    let mut expr = expr.clone();
    // Distinct source callbacks need distinct emitted names: a nested renderer may
    // capture its parent's children. Hygiene is disabled, so contexts alone do not help.
    let props: Ident = format!("$jg$intlChildren{}", expr.span().lo.0).into();
    let params = match &expr {
      Expr::Arrow(f) if !f.is_async && !f.is_generator => Some(f.params.clone()),
      Expr::Fn(f) if !f.function.is_async && !f.function.is_generator => {
        Some(f.function.params.iter().map(|p| p.pat.clone()).collect())
      }
      Expr::Arrow(_) | Expr::Fn(_) => {
        emit_error(expr.span(), "国际化富文本回调不支持 async/generator");
        return None;
      }
      _ => None,
    };
    if let Some(params) = params {
      if params.len() > 1 || params.first().is_some_and(|p| !matches!(p, Pat::Ident(_))) {
        emit_error(
          expr.span(),
          "国际化富文本回调只支持零个参数或一个普通 children 参数，不支持默认值、解构或 rest",
        );
        return None;
      }
      if let Expr::Fn(function) = &expr {
        struct Arguments(bool);
        impl Visit for Arguments {
          fn visit_function(&mut self, _: &Function) {}
          fn visit_ident(&mut self, id: &Ident) {
            self.0 |= id.sym == *"arguments";
          }
        }
        let mut arguments = Arguments(false);
        function.function.body.visit_with(&mut arguments);
        if arguments.0 {
          emit_error(
            expr.span(),
            "国际化富文本回调不支持读取自身 arguments，请使用 children 参数",
          );
          return None;
        }
      }
      if let Some(Pat::Ident(child)) = params.first() {
        let body = match &expr {
          Expr::Arrow(f) => (*f.body).clone(),
          Expr::Fn(f) => ArrowFunctionBody::FunctionBody(f.function.body.clone()?),
          _ => unreachable!(),
        };
        if crate::scope::binding_is_written(&body, &child.id.to_id()) {
          emit_error(child.id.span, "国际化富文本回调不能重新赋值 children 参数");
          return None;
        }
        struct Children {
          binding: Id,
          props: Ident,
        }
        impl Children {
          fn replacement(&self) -> Box<Expr> {
            ast_create_expr_member(
              ast_create_expr_ident(self.props.clone()),
              MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: crate::ast::ast_create_expr_lit_str("slot:default".into()),
              }),
            )
          }
        }
        impl VisitMut for Children {
          fn visit_mut_expr(&mut self, node: &mut Expr) {
            if matches!(node, Expr::Ident(id) if id.to_id() == self.binding) {
              *node = *self.replacement();
            } else {
              node.visit_mut_children_with(self);
            }
          }
          fn visit_mut_prop(&mut self, node: &mut Prop) {
            if let Prop::Shorthand(id) = node
              && id.to_id() == self.binding
            {
              *node = Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(id.clone().into()),
                value: self.replacement(),
              });
            } else {
              node.visit_mut_children_with(self);
            }
          }
        }
        expr.visit_mut_with(&mut Children {
          binding: child.id.to_id(),
          props: props.clone(),
        });
      }
      match &mut expr {
        Expr::Arrow(f) => f.params = vec![Pat::Ident(props.into())],
        Expr::Fn(f) => f.function.params = vec![Param::from(Pat::Ident(props.into()))],
        _ => unreachable!(),
      }
    }
    let mut parser = TemplateParser::new(
      self.props_arg.clone(),
      Some(super::JINGE_HOST_IDENT.clone()),
      self.intl_type,
    );
    // Rich renderers retain outer map bindings; nested loops must not reuse their names.
    parser.map_loop_level = self.map_loop_level;
    parser.visit_expr(&expr);
    let slot = parser.context.slots.pop().unwrap();
    Some(super::component::slot_to_expr(
      slot.params,
      slot.stmts,
      slot.expressions,
      slot.function,
    ))
  }

  /// Recognize the current file's direct jinge t import by resolver binding Id.
  pub fn parse_intl_t(&mut self, callee: &Expr, args: &[ExprOrSpread]) -> bool {
    if !matches!(callee, Expr::Ident(name) if crate::scope::is_intl_t(name)) {
      return false;
    }
    let Some((key, default_text, params_arg)) = extract_t(args) else {
      return false;
    };

    let default_text_param = if matches!(self.intl_type, IntlType::Enabled(true)) {
      None
    } else {
      Some(default_text)
    };
    let host_ident = self.create_host_ident();

    let Some(params) = params_arg else {
      self.push_expression(tpl_render_intl_normal_text(
        key,
        None,
        default_text_param,
        self.context.is_parent_component(),
        host_ident.clone(),
      ));
      return true; // 如果没有 params 参数，生成简单的 renderIntlText 函数。
    };
    let is_parent = self.context.is_parent_component();
    let mut vm = IntlParams {
      is_rich_text: false,
      const_props: vec![],
      watch_props: vec![],
    };
    for prop in params.props.iter() {
      match prop {
        PropOrSpread::Spread(_) => {
          emit_error(prop.span(), "t 函数的 params 参数不支持 ... 解构写法");
          self.push_expression(tpl_render_intl_normal_text(
            key,
            None,
            default_text_param,
            is_parent,
            host_ident.clone(),
          ));
          return true;
        }
        PropOrSpread::Prop(prop) => {
          let Prop::KeyValue(kv) = prop.as_ref() else {
            emit_error(
              prop.span(),
              "t 函数的 params 参数必须是 key-value 类型的 Object。",
            );
            self.push_expression(tpl_render_intl_normal_text(
              key,
              None,
              default_text_param,
              is_parent,
              host_ident.clone(),
            ));
            return true;
          };

          match kv.value.as_ref() {
            Expr::JSXElement(_)
            | Expr::JSXEmpty(_)
            | Expr::JSXFragment(_)
            | Expr::JSXMember(_)
            | Expr::JSXNamespacedName(_)
            | Expr::Arrow(_)
            | Expr::Fn(_) => {
              // jsx 或者函数都认为是富文本格式的组件
              vm.is_rich_text = true;
              let tag = match &kv.key {
                PropName::Ident(id) => id.sym.to_string(),
                PropName::Str(s) => s.value.to_string_lossy().into_owned(),
                _ => {
                  emit_error(kv.key.span(), "国际化富文本标签名必须是静态名称");
                  continue;
                }
              };
              if let Some(renderer) = self.parse_intl_renderer(&kv.value) {
                vm.const_props
                  .push((PropName::Str(Str::from(format!("slot:{tag}"))), renderer));
              }
            }
            Expr::Lit(val) => {
              vm.const_props
                .push((kv.key.clone(), Box::new(Expr::Lit(val.clone()))));
            }

            _ => {
              let r = ExprVisitor::new(self.intl_type).parse(kv.value.as_ref());
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

    let has_const_props = !vm.const_props.is_empty();
    let has_watch_props = !vm.watch_props.is_empty();

    let params_props = Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vm
        .const_props
        .into_iter()
        .map(|(prop, value)| {
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp { key: prop, value })))
        })
        .collect(),
    }));

    if !has_watch_props {
      let expr = tpl_render_intl_text(
        vm.is_rich_text,
        key,
        if has_const_props {
          Some(ExprOrSpread {
            spread: None,
            expr: params_props,
          })
        } else {
          None
        },
        default_text_param,
        is_parent,
        host_ident.clone(),
      );
      if vm.is_rich_text {
        self.push_expression_with_spread(expr);
      } else {
        self.push_expression(expr);
      }
      return true;
    }

    let mut stmts = vec![ast_create_stmt_decl_const(
      JINGE_ATTR_IDENT.clone(),
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_VM.local()),
        vec![ast_create_arg_expr(params_props)],
      ),
    )];

    vm.watch_props
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
          expr: tpl_watch_and_render(set_fn, watch_expr, host_ident.clone()),
        }));
      });

    stmts.push(Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(tpl_render_intl_text(
        vm.is_rich_text,
        key,
        Some(ExprOrSpread {
          spread: None,
          expr: ast_create_expr_ident(JINGE_ATTR_IDENT.clone()),
        }),
        default_text_param,
        is_parent,
        host_ident,
      )),
    }));

    let expr = ast_create_expr_call(
      ast_create_expr_arrow_fn(
        vec![],
        Box::new(ArrowFunctionBody::FunctionBody(FunctionBody {
          span: DUMMY_SP,
          stmts,
        })),
      ),
      vec![],
    );
    if vm.is_rich_text {
      self.push_expression_with_spread(expr);
    } else {
      self.push_expression(expr);
    }

    true
  }
}

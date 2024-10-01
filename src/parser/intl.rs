use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha512};
use swc_common::{Spanned, SyntaxContext, DUMMY_SP};
use swc_core::{atoms::Atom, ecma::ast::*};

use super::{
  ast_create_arg_expr, ast_create_expr_arrow_fn, ast_create_expr_call, ast_create_expr_ident,
  ast_create_stmt_decl_const, emit_error,
  expr::{ExprParseResult, ExprVisitor},
  tpl_render_intl_normal_text, tpl_render_intl_text, tpl_watch_and_render, IntlType,
  TemplateParser, JINGE_ATTR_IDENT, JINGE_IMPORT_VM, JINGE_KEY, JINGE_T, JINGE_V_IDENT,
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

pub fn extract_t<'a>(
  args: &'a Vec<ExprOrSpread>,
) -> Option<(Atom, &'a Atom, Option<&'a ObjectLit>)> {
  let Some(default_text) = args.get(0) else {
    return None;
  };
  if default_text.spread.is_some() {
    return None;
  }
  let Expr::Lit(Lit::Str(default_text)) = default_text.expr.as_ref() else {
    emit_error(
      default_text.span(),
      "t 函数的第一个参数必须是字符串常量，代表默认文本",
    );
    return None;
  };

  let default_text = &default_text.value;

  let mut key = None;
  // let mut isolate = false;
  let options_arg = args.get(2);
  if let Some(options) = options_arg {
    if options.spread.is_some() {
      emit_error(options.span(), "t 函数的 options 参数不支持 ... 解构写法");
    } else if let Expr::Object(opts) = options.expr.as_ref() {
      for prop in opts.props.iter() {
        if let PropOrSpread::Prop(prop) = prop {
          if let Prop::KeyValue(kv) = prop.as_ref() {
            if let PropName::Ident(id) = &kv.key {
              if JINGE_KEY.eq(&id.sym) {
                if let Expr::Lit(Lit::Str(v)) = kv.value.as_ref() {
                  key = Some(v.value.clone());
                }
                // 找到 key 就退出循环。
                break;
              }
            }
          }
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
    key = Some(calc_intl_key(default_text.as_str(), None).into());
  }

  Some((key.unwrap(), default_text, params_arg))
}
impl TemplateParser {
  /// 将国际化多语言的 t 函数转换为相应的组件或渲染。这里采用了极简单的粗糙方法，仅通过函数名为 t 来判定。
  /// 因此有很大的问题，比如不支持 `import {t as someFn} from 'jinge'` 的别名 import 写法；
  /// 比如如果用户使用了自已定义的也名为 t 函数。
  /// TODO: 未来结合实现情况来支持上述两种 case。
  pub fn parse_intl_t(&mut self, callee: &Expr, args: &Vec<ExprOrSpread>) -> bool {
    if !matches!(callee, Expr::Ident(name) if JINGE_T.eq(&name.sym)) {
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

    let Some(params) = params_arg else {
      self.push_expression(tpl_render_intl_normal_text(
        key,
        None,
        default_text_param,
        self.context.is_parent_component(),
        self.context.root_container,
      ));
      return true; // 如果没有 params 参数，生成简单的 renderIntlText 函数。
    };

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
            self.context.is_parent_component(),
            self.context.root_container,
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
              self.context.is_parent_component(),
              self.context.root_container,
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
            }
            Expr::Lit(val) => {
              vm.const_props
                .push((kv.key.clone(), Box::new(Expr::Lit(val.clone()))));
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
        self.context.is_parent_component(),
        self.context.root_container,
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
          expr: tpl_watch_and_render(set_fn, watch_expr, self.context.root_container),
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
        self.context.is_parent_component(),
        self.context.root_container,
      )),
    }));

    let expr = ast_create_expr_call(
      ast_create_expr_arrow_fn(
        vec![],
        Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
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

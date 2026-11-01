use swc_common::{Spanned, SyntaxContext};
use swc_ecma_visit::VisitWith;

use crate::parser::{attrs::AttrWatchPropName, *};

use super::{TemplateParser, emit_error};

// SVG-specific intrinsic tags can be identified even across component/slot boundaries.
// Shared HTML/SVG names (a, title, style, script) still require the existing context.
fn is_svg_tag(name: &str) -> bool {
  matches!(
    name,
    "svg"
      | "animate"
      | "animateMotion"
      | "animateTransform"
      | "circle"
      | "clipPath"
      | "defs"
      | "desc"
      | "ellipse"
      | "feBlend"
      | "feColorMatrix"
      | "feComponentTransfer"
      | "feComposite"
      | "feConvolveMatrix"
      | "feDiffuseLighting"
      | "feDisplacementMap"
      | "feDistantLight"
      | "feDropShadow"
      | "feFlood"
      | "feFuncA"
      | "feFuncB"
      | "feFuncG"
      | "feFuncR"
      | "feGaussianBlur"
      | "feImage"
      | "feMerge"
      | "feMergeNode"
      | "feMorphology"
      | "feOffset"
      | "fePointLight"
      | "feSpecularLighting"
      | "feSpotLight"
      | "feTile"
      | "feTurbulence"
      | "filter"
      | "foreignObject"
      | "g"
      | "image"
      | "line"
      | "linearGradient"
      | "marker"
      | "mask"
      | "metadata"
      | "mpath"
      | "path"
      | "pattern"
      | "polygon"
      | "polyline"
      | "radialGradient"
      | "rect"
      | "set"
      | "stop"
      | "switch"
      | "symbol"
      | "text"
      | "textPath"
      | "tspan"
      | "use"
      | "view"
  )
}

impl TemplateParser {
  fn parse_html_element(&mut self, tn: &Ident, n: &JSXElement) {
    let mut attrs = self.parse_attrs(n, false);
    let is_svg = self.context.is_parent_svg() || is_svg_tag(tn.sym.as_ref());
    self.push_context_inherit_host_ident(if is_svg && tn.sym != "foreignObject" {
      Parent::Svg
    } else {
      Parent::Html
    });
    // println!("meet html {} {}", tn.sym.as_str(), self.context.slot_level);
    // 此处不能直接用 n.visit_children_with(self)，会再次 visit attributes
    n.children.iter().for_each(|child| {
      child.visit_children_with(self);
    });
    let mut children_context = self.pop_context();
    // html 元素下不可能出现多个 slots。事实上，html 元素没有 slot 概念，只是用统一的数据结构保存子节点。
    assert_eq!(children_context.slots.len(), 1);
    let had_const_props = !attrs.const_props.is_empty();
    let callee_ident = if is_svg {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_SVG_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_SVG_ELE.local()
      }
    } else {
      if !attrs.const_props.is_empty() {
        JINGE_IMPORT_CREATE_ELE_A.local()
      } else {
        JINGE_IMPORT_CREATE_ELE.local()
      }
    };
    let mut args = vec![ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(
      Str::from(tn.sym.clone()),
    ))))];
    let host_ident = self.create_host_ident();

    let mut idl_props = vec![];
    if !is_svg {
      attrs.const_props.retain(|(name, value)| {
        if IDL_ATTRIBUTE_SET.binary_search(&name.sym).is_ok() {
          idl_props.push((name.clone(), value.clone()));
          false
        } else {
          true
        }
      });
    }
    // Select mode must be established before selected options are appended.
    // Keep value and other property bindings in their existing post-child phase.
    let initialize_multiple_first = !is_svg
      && tn.sym == "select"
      && (idl_props.iter().any(|(name, _)| name.sym == "multiple")
        || attrs.watch_props.iter().any(
          |(name, _)| matches!(name, AttrWatchPropName::Id(name) if name.as_str() == "multiple"),
        ));
    let (initial_idl_props, idl_props): (Vec<_>, Vec<_>) = if initialize_multiple_first {
      idl_props
        .into_iter()
        .partition(|(name, _)| name.sym == "multiple")
    } else {
      (Vec::new(), idl_props)
    };
    let initial_watch_props = if initialize_multiple_first {
      let (initial, remaining): (Vec<_>, Vec<_>) = std::mem::take(&mut attrs.watch_props)
        .into_iter()
        .partition(
          |(name, _)| matches!(name, AttrWatchPropName::Id(name) if name.as_str() == "multiple"),
        );
      attrs.watch_props = remaining;
      initial
    } else {
      Vec::new()
    };
    let set_ref_code = attrs
      .ref_prop
      .take()
      .map(|r| tpl_set_ref_code(r, host_ident.clone()));
    let push_ele_code = if self.context.is_parent_component() {
      Some(tpl_push_el_code(true, host_ident.clone()))
    } else {
      None
    };
    if had_const_props {
      args.push(ast_create_arg_expr(tpl_lit_obj(attrs.const_props)));
    }
    let mut children = std::mem::take(&mut children_context.slots[0].expressions);
    if !initialize_multiple_first {
      args.append(&mut children);
    }

    let output = if set_ref_code.is_some()
      || push_ele_code.is_some()
      || !attrs.dom_const_events.is_empty()
      || !attrs.dom_watch_events.is_empty()
      || !attrs.watch_props.is_empty()
      || !idl_props.is_empty()
      || initialize_multiple_first
    {
      let mut stmts: Vec<Stmt> = vec![ast_create_stmt_decl_const(
        JINGE_EL_IDENT.clone(),
        ast_create_expr_call(ast_create_expr_ident(callee_ident), args),
      )];
      for (name, value) in initial_idl_props {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_assign_mem(
            ast_create_expr_ident(JINGE_EL_IDENT.clone()),
            name.sym,
            value,
          ),
        }));
      }
      for (name, watch_expr) in initial_watch_props {
        let AttrWatchPropName::Id(name) = name else {
          unreachable!()
        };
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_set_html_attr(name, watch_expr, host_ident.clone()),
        }));
      }
      if initialize_multiple_first && !children.is_empty() {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(
            ast_create_expr_member(
              ast_create_expr_ident(JINGE_EL_IDENT.clone()),
              MemberProp::Ident(IdentName::from("append")),
            ),
            children,
          ),
        }));
      }
      for (name, value) in idl_props {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_assign_mem(
            ast_create_expr_ident(JINGE_EL_IDENT.clone()),
            name.sym,
            value,
          ),
        }));
      }
      attrs.dom_const_events.into_iter().for_each(|evt| {
        let mut args = vec![
          ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
          ast_create_arg_expr(ast_create_expr_lit_str(evt.event_name.sym.into())),
          ast_create_arg_expr(evt.event_handler),
        ];
        if evt.capture {
          args.push(ast_create_arg_expr(ast_create_expr_lit_bool(true)));
        }
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_ADD_EVENT.local()), args),
        }))
      });
      attrs.dom_watch_events.into_iter().for_each(|evt| {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: tpl_watch_and_bind_html_event(
            evt.event_name,
            evt.event_handler,
            host_ident.clone(),
            evt.capture,
          ),
        }));
      });
      attrs
        .watch_props
        .into_iter()
        .for_each(|(attr_name, watch_expr)| {
          stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: tpl_watch_and_set_html_attr(
              match attr_name {
                AttrWatchPropName::Id(attr_name) => attr_name,
                _ => unreachable!(),
              },
              watch_expr,
              host_ident.clone(),
            ),
          }));
        });
      if let Some(c) = set_ref_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      if let Some(c) = push_ele_code {
        stmts.push(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: c,
        }))
      }
      stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      }));
      let body = Box::new(ArrowFunctionBody::FunctionBody(FunctionBody {
        span: DUMMY_SP,
        stmts,
      }));
      let callee = Box::new(Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(Expr::Arrow(ArrowExpr {
          ctxt: SyntaxContext::empty(),
          span: DUMMY_SP,
          params: vec![],
          body,
          is_async: false,
          is_generator: false,
          type_params: None,
          return_type: None,
        })),
      }));
      ast_create_expr_call(callee, vec![])
    } else {
      ast_create_expr_call(ast_create_expr_ident(callee_ident), args)
    };
    // 当前 html 元素添加到父亲的最顶部 Slot 中。最顶部 Slot 可能是默认 Slot(比如父亲也是 html 元素则也是存放在默认 Slot)，也可能是命名 Slot(只可能出现在父亲是组件的情况)
    self
      .context
      .slots
      .last_mut()
      .unwrap()
      .expressions
      .push(ExprOrSpread {
        spread: None,
        expr: output,
      });
  }

  pub fn parse_jsx_element(&mut self, n: &JSXElement) {
    let JSXElementName::Ident(tn) = &n.opening.name else {
      emit_error(
        n.opening.name.span(),
        "未知的 JSX 格式，opening.name 未找到",
      );
      return;
    };
    // let tag = tn.as_ref();
    // println!("visit jsx ele: {}", tn.as_ref());
    match tn.as_ref().chars().next() {
      Some(c) if c.is_ascii_uppercase() => {
        self.parse_component_element(tn, n);
      }
      Some(c) if c.is_ascii_lowercase() => {
        self.parse_html_element(tn, n);
      }
      _ => {
        emit_error(
          tn.span(),
          "不支持的 Tag。合法 Tag 为：大写字母打头为 Component 组件，小写字母打头为 html 元素。",
        );
      }
    }
  }
}

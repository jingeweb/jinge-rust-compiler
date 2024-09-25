use swc_core::{
  atoms::Atom,
  common::{SyntaxContext, DUMMY_SP},
  ecma::ast::*,
};

use crate::{ast::*, common::*};

use super::expr::ExprParseResult;

pub fn tpl_set_ref_code(r: Box<Expr>) -> Box<Expr> {
  let args = vec![
    ast_create_arg_expr(ast_create_expr_this()),
    ast_create_arg_expr(r),
    ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
  ];

  ast_create_expr_call(ast_create_expr_ident(JINGE_IMPORT_SET_REF.local()), args)
}

pub fn tpl_push_el_code(root: bool, is_root_container: bool) -> Box<Expr> {
  let args = vec![ast_create_arg_expr(ast_create_expr_ident(
    JINGE_EL_IDENT.clone(),
  ))];
  Box::new(Expr::Call(CallExpr {
    ctxt: SyntaxContext::empty(),
    span: DUMMY_SP,
    callee: Callee::Expr(ast_create_expr_member(
      ast_create_expr_member(
        ast_create_id_of_container(is_root_container),
        MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: ast_create_expr_ident(if root {
            JINGE_IMPORT_ROOT_NODES.local()
          } else {
            JINGE_IMPORT_NON_ROOT_COMPONENT_NODES.local()
          }),
        }),
      ),
      MemberProp::Ident(IdentName::from("push")),
    )),
    args,
    type_args: None,
  }))
}

pub fn tpl_lit_obj(lit_arr: Vec<(IdentName, Box<Expr>)>) -> Box<Expr> {
  Box::new(Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: lit_arr
      .into_iter()
      .map(|(prop, value)| {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Str(Str::from(prop.sym)),
          value,
        })))
      })
      .collect(),
  }))
}

pub fn tpl_set_attribute(el: Box<Expr>, attr_name: Atom, attr_value: Box<Expr>) -> Box<Expr> {
  ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_SET_ATTRIBUTE.local()),
    vec![
      ast_create_arg_expr(el),
      ast_create_arg_expr(Box::new(Expr::Lit(Lit::Str(Str::from(attr_name))))),
      ast_create_arg_expr(attr_value),
    ],
  )
}

pub fn tpl_render_const_text(
  c: Box<Expr>,
  is_parent_component: bool,
  is_root_container: bool,
) -> Box<Expr> {
  if is_parent_component {
    ast_create_expr_call(
      ast_create_expr_ident(JINGE_IMPORT_TEXT_RENDER_FN.local()),
      vec![
        ast_create_arg_expr(ast_create_id_of_container(is_root_container)),
        ast_create_arg_expr(c),
      ],
    )
  } else {
    c
  }
}

pub fn tpl_render_intl_text(
  key: &Atom,
  params: Option<&ExprOrSpread>,
  default_text: Option<&Atom>,
  is_parent_component: bool,
  is_root_container: bool,
) -> Box<Expr> {
  println!("{}, {}", is_parent_component, is_root_container);
  let mut args = vec![
    ast_create_arg_expr(ast_create_id_of_container(is_root_container)),
    ast_create_arg_expr(ast_create_expr_lit_bool(is_parent_component)),
    ast_create_arg_expr(ast_create_expr_lit_str(key.clone())),
  ];
  if let Some(params) = params {
    args.push(params.clone());
    println!("{:?}", params);
  }
  if let Some(default_text) = default_text {
    if params.is_none() {
      args.push(ast_create_arg_expr(Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![],
      }))));
    }
    args.push(ast_create_arg_expr(ast_create_expr_lit_str(
      default_text.clone(),
    )));
  }
  ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_RENDER_INTL_TEXT.local()),
    args,
  )
}

pub fn tpl_render_expr_text(
  expr_result: ExprParseResult,
  value: Box<Expr>,
  is_parent_component: bool,
  is_root_container: bool,
) -> Box<Expr> {
  let render_fn = ast_create_expr_call(
    ast_create_expr_ident(JINGE_IMPORT_SET_TEXT_CONTENT.local()),
    vec![
      ast_create_arg_expr(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
      ast_create_arg_expr(value),
    ],
  );

  let mut stmts = vec![
    ast_create_stmt_decl_const(
      Ident::from(JINGE_EL_IDENT.clone()),
      ast_create_expr_call(
        ast_create_expr_ident(Ident::from(JINGE_IMPORT_CREATE_TEXT_NODE.local())),
        vec![ast_create_arg_expr(ast_create_expr_lit_str(
          JINGE_EMPTY_STR.clone(),
        ))],
      ),
    ),
    Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_watch_and_render(render_fn, expr_result, is_root_container),
    }),
  ];

  if is_parent_component {
    stmts.push(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: tpl_push_el_code(true, is_root_container),
    }));
  }
  stmts.push(Stmt::Return(ReturnStmt {
    span: DUMMY_SP,
    arg: Some(ast_create_expr_ident(JINGE_EL_IDENT.clone())),
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

pub fn tpl_watch_and_render(
  render_fn_body: Box<Expr>,
  expr_result: ExprParseResult,
  is_root_container: bool,
) -> Box<Expr> {
  match expr_result {
    ExprParseResult::None => unreachable!(),
    ExprParseResult::Complex(watch_expr) => {
      let args = vec![
        ast_create_arg_expr(watch_expr),
        ast_create_arg_expr(ast_create_expr_arrow_fn(
          vec![Pat::Ident(BindingIdent::from(JINGE_V_IDENT.clone()))],
          Box::new(BlockStmtOrExpr::Expr(render_fn_body)),
        )),
        // 复杂表达式，会有 PathWatcher/ExprWatcher 等的封装，统一加到 [HOST_WATCH] 中，在 host component 销毁时卸载。
        ast_create_arg_expr(ast_create_id_of_container(is_root_container)),
      ];
      ast_create_expr_call(
        ast_create_expr_ident(JINGE_IMPORT_WATCH_FOR_RENDER.local()),
        args,
      )
    }
    ExprParseResult::Simple(sr) => {
      let mut args = vec![
        ast_create_arg_expr(sr.vm),
        ast_create_arg_expr(sr.path),
        ast_create_arg_expr(ast_create_expr_arrow_fn(
          vec![Pat::Ident(BindingIdent::from(JINGE_V_IDENT.clone()))],
          Box::new(BlockStmtOrExpr::Expr(render_fn_body)),
        )),
      ];
      if sr.not_op > 0 {
        args.push(ast_create_arg_expr(Box::new(Expr::Lit(Lit::Num(
          Number::from(sr.not_op as usize),
        )))));
      }
      if !is_root_container || !sr.is_this {
        // 简单表达式，如果不是 root container，说明一定有 Slot 传递的 host component，需要添加 [HOST WATCH]
        // 如果表达式不是 this. 打头的，也就是监听的可能是全局 vm，或监听的 Slot 传递进来的 vm。
        // 如果监听全局 vm，则要把对全局变量 vm 的监听放到 this 组件的 [HOST_WATCH]中，在 this 组件销毁时卸载。
        // 如果监听的传递进来的 vm ，也同理。
        args.push(ast_create_arg_expr(ast_create_id_of_container(
          is_root_container,
        )));
      }
      ast_create_expr_call(
        ast_create_expr_ident(if sr.not_op > 0 {
          JINGE_IMPORT_WATCH_PATH_FOR_RENDER_2.local()
        } else {
          JINGE_IMPORT_WATCH_PATH_FOR_RENDER.local()
        }),
        args,
      )
    }
  }
}

pub fn tpl_watch_and_set_html_attr(
  attr_name: IdentName,
  expr_result: ExprParseResult,
  is_root_container: bool,
) -> Box<Expr> {
  let set_fn = if IDL_ATTRIBUTE_SET.binary_search(&attr_name.sym).is_ok() {
    ast_create_expr_assign_mem(
      ast_create_expr_ident(JINGE_EL_IDENT.clone()),
      attr_name.sym,
      ast_create_expr_ident(JINGE_V_IDENT.clone()),
    )
  } else {
    tpl_set_attribute(
      ast_create_expr_ident(JINGE_EL_IDENT.clone()),
      attr_name.sym,
      ast_create_expr_ident(JINGE_V_IDENT.clone()),
    )
  };
  tpl_watch_and_render(set_fn, expr_result, is_root_container)
}

pub fn tpl_watch_and_set_component_attr(
  attr_name: IdentName,
  expr_result: ExprParseResult,
  is_root_container: bool,
) -> Box<Expr> {
  let set_fn = ast_create_expr_assign_mem(
    ast_create_expr_ident(JINGE_ATTR_IDENT.clone()),
    attr_name.sym,
    ast_create_expr_ident(JINGE_V_IDENT.clone()),
  );
  tpl_watch_and_render(set_fn, expr_result, is_root_container)
}

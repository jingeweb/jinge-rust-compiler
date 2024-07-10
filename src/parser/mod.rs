use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::Visit;

mod tpl;

enum Parent {
  Component,
  Html(Html),
  Null,
}

struct Html {
  is_svg: bool,
}

struct Context {
  parent: Parent,
  expressions: Box<Vec<Box<Expr>>>,
}

pub struct TemplateParser {
  context: Context,
  stack: Vec<Context>,
}

impl TemplateParser {
  pub fn new() -> Self {
    let root_context = Context {
      parent: Parent::Null,
      expressions: Box::new(vec![]),
    };
    Self {
      context: root_context,
      stack: vec![],
    }
  }
  fn push_context(&mut self, p: Parent) {
    let current_context = std::mem::replace(
      &mut self.context,
      Context {
        parent: p,
        expressions: Box::new(vec![]),
      },
    );
    self.stack.push(current_context);
  }
  fn pop_context(&mut self) -> Context {
    std::mem::replace(&mut self.context, self.stack.pop().unwrap())
  }
}

impl Visit for TemplateParser {
  fn visit_lit(&mut self, n: &Lit) {
    if let Parent::Html(_) = &self.context.parent {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.context.expressions.push(Box::new(e));
    } else {
      let mut e = Expr::Lit(n.clone());
      e.set_span(DUMMY_SP);
      self.context.expressions.push(gen_text_render_func(Box::new(e)));
    }
  }
  fn visit_jsx_text(&mut self, n: &JSXText) {
    let text = n.value.trim();
    if !text.is_empty() {
      self.context.expressions.push(ast_create_expr_lit_str(text));
    }
  }

  fn visit_expr(&mut self, n: &Expr) {
    match n {
      Expr::JSXElement(n) => {
        let JSXElementName::Ident(tn) = &n.opening.name else {
          emit_error(n.opening.name.span(), "todo");
          return;
        };
        let tag = tn.as_ref();
        match tag.chars().next() {
          Some(c) if c >= 'A' && c <= 'Z' => {}
          Some(c) if c >= 'a' && c <= 'z' => {
            let mut a_ref: Option<Atom> = None;
            let mut a_lits: Vec<(Ident, Lit)> = vec![];
            n.opening.attrs.iter().for_each(|attr| match attr {
              JSXAttrOrSpread::SpreadElement(s) => {
                emit_error(s.span(), "暂不支持 ... 属性");
              }
              JSXAttrOrSpread::JSXAttr(attr) => {
                let JSXAttrName::Ident(n) = &attr.name else {
                  return;
                };
                let name = &n.sym;
                if name == "ref" {
                  if a_ref.is_some() {
                    emit_error(attr.span(), "不能重复指定 ref");
                    return;
                  }
                  a_ref.replace(name.clone());
                } else if name.starts_with("on")
                  && matches!(name.chars().nth(2), Some(c) if c >= 'A' && c <= 'Z')
                {
                  // html event
                } else {
                  if let Some(val) = &attr.value {
                    match val {
                      JSXAttrValue::Lit(val) => {
                        a_lits.push((tn.clone(), val.clone()));
                      }
                      JSXAttrValue::JSXExprContainer(val) => match &val.expr {
                        JSXExpr::JSXEmptyExpr(_) => {
                          emit_error(val.expr.span(), "属性值为空");
                        }
                        JSXExpr::Expr(expr) => match expr.as_ref() {
                          Expr::JSXElement(_)
                          | Expr::JSXEmpty(_)
                          | Expr::JSXFragment(_)
                          | Expr::JSXMember(_)
                          | Expr::JSXNamespacedName(_) => {
                            emit_error(val.expr.span(), "不支持 JSX 元素作为属性值");
                          }
                          Expr::Lit(val) => {
                            a_lits.push((tn.clone(), val.clone()));
                          }
                          _ => {
                            // expr attribute
                          }
                        },
                      },
                      _ => emit_error(val.span(), "不支持该类型的属性值。"),
                    }
                  } else {
                    // bool attribute
                    a_lits.push((tn.clone(), Lit::Bool(Bool::from(true))));
                  }
                }
              }
            });

            let is_svg = tag == "svg";
            self.push_context(Parent::Html(Html { is_svg }));
            n.visit_children_with(self);
            let context = self.pop_context();
            let mut args = vec![ExprOrSpread {
              spread: None,
              expr: ast_create_expr_lit_str(tag),
            }];
            if !context.expressions.is_empty() {
              args.append(
                &mut context
                  .expressions
                  .into_iter()
                  .map(|expr| ExprOrSpread { spread: None, expr })
                  .collect::<Vec<ExprOrSpread>>(),
              );
            }
            self.context.expressions.push(ast_create_expr_call(
              ast_create_expr_ident(JINGE_IMPORT_CREATE_ELE.1),
              args,
            ));
          }
          _ => {
            emit_error(
              tn.span(),
              "不支持的 Tag。合法 Tag 为：大写字母打头为 Component 组件，小写字母打头为 html 元素。",
            );
            return;
          }
        }
      }
      Expr::JSXEmpty(_) => (),
      Expr::JSXFragment(n) => {
        if n.children.is_empty() {
          return;
        }
      }
      Expr::JSXMember(n) => {
        emit_error(n.span(), "todo");
      }
      Expr::JSXNamespacedName(n) => {
        emit_error(n.span(), "todo");
      }
      Expr::Call(n) => emit_error(n.span(), "不支持函数调用"),
      Expr::Cond(_) => {
        emit_error(n.span(), "不支持二元条件表达式，请使用 <If> 组件");
      }
      Expr::Bin(b) => match b.op {
        BinaryOp::Add
        | BinaryOp::Exp
        | BinaryOp::Sub
        | BinaryOp::Mul
        | BinaryOp::LShift
        | BinaryOp::RShift
        | BinaryOp::ZeroFillRShift
        | BinaryOp::Mod
        | BinaryOp::Div
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor => b.visit_children_with(self),
        _ => emit_error(b.span(), "不支持条件表达式，请使用 <If> 组件"),
      },
      _ => {
        n.visit_children_with(self);
      }
    }
  }
}

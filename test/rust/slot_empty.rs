use super::*;
fn compile_slot(source: &str) -> String {
  let (code, _, _) = inner_transform(
    "slot-empty.tsx".into(),
    2,
    source.into(),
    false,
    IntlType::Disabled,
  )
  .unwrap();
  assert_javascript(&code);
  code
}
#[test]
fn absent_slot_guard_emits_a_comment_registered_to_the_custom_host() {
  let code =
    compile_slot("export function A(props,hostCustom){return <>{props.children&&<b>yes</b>}</>}");
  assert!(code.contains("createComment$jg$("), "{code}");
  assert!(
    code
      .matches("hostCustom[ROOT_NODES$jg$].push($jg$)")
      .count()
      >= 2,
    "{code}"
  );
}
#[test]
fn empty_slot_ternary_and_fallback_branches_emit_comments() {
  for expression in [
    "props.children?<b/>:<></>",
    "props.children?<></>:<b/>",
    "props.children||<></>",
    "props.children??<></>",
  ] {
    let code = compile_slot(&format!(
      "export function A(props){{return <>{{{expression}}}</>}}"
    ));
    assert!(code.contains("createComment$jg$("), "{expression}:\n{code}");
  }
}
#[test]
fn unrelated_templates_do_not_import_the_optional_comment_helper() {
  let code = compile_slot("export function A(){return <p>hi</p>}");
  assert!(!code.contains("createComment"), "{code}");
}

#[test]
fn f05_chained_slots_render_each_branch_as_a_slot() {
  for expression in [
    "props.children ?? props['slot:icon'] ?? 'fallback'",
    "(props.children ?? props['slot:icon']) ?? props['slot:extra'] ?? 'fallback'",
    "props.children?.() ?? props['slot:icon']?.() ?? 'fallback'",
  ] {
    let code = compile_slot(&format!(
      "export function A(props){{return <>{{{expression}}}</>}}"
    ));
    assert!(code.contains("\"slot:default\""), "{code}");
    assert!(code.contains("renderSlotFunction$jg$("), "{code}");
    assert!(!code.contains("ExprWatcher$jg$("), "{code}");
    assert!(!code.contains("props.children"), "{code}");
  }
}

#[test]
fn f14_data_children_are_not_slot_components() {
  let code = compile_slot(
    "export function A(){const config={children:node=>node.children};const children=config.children({children:[{label:'child'}]});return <span>{children[0].label}</span>}",
  );
  assert!(code.contains("node.children"), "{code}");
  assert!(!code.contains("slot:default"), "{code}");
  assert!(!code.contains("renderSlotFunction$jg$("), "{code}");
}

#[test]
fn f14_children_matches_the_props_binding_not_its_spelling() {
  GLOBALS.set(&Globals::default(), || {
    use swc_common::{DUMMY_SP, SyntaxContext};
    use swc_core::ecma::ast::{Expr, Ident, MemberExpr, MemberProp};
    let props = Ident::new(
      "input".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(Mark::new()),
    );
    let shadow = Ident::new(
      "input".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(Mark::new()),
    );
    let mut member = MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(shadow)),
      prop: MemberProp::Ident("children".into()),
    };
    assert!(
      crate::parser::slot::get_slot_name_from_member_expr(&member, &Some(props.clone())).is_none()
    );
    member.obj = Box::new(Expr::Ident(props.clone()));
    assert!(crate::parser::slot::get_slot_name_from_member_expr(&member, &Some(props)).is_some());
  });
}

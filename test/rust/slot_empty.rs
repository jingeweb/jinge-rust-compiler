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

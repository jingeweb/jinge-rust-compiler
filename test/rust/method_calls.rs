use super::*;
fn method_code(source: &str) -> String {
  let (code, _, _) = inner_transform(
    "method.tsx".into(),
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
fn member_calls_use_a_specialized_watcher_without_a_deep_flag() {
  let code =
    method_code("export function A(props){return <p>{props.service.label(props.suffix)}</p>}");
  assert!(code.contains("MethodPathWatcher$jg$(props"), "{code}");
  assert!(code.contains("$jg$a0($jg$a1)"), "{code}");
  assert!(
    !code.contains("\"label\"\n        ], true"),
    "method receiver contents must not be watched: {code}"
  );
}
#[test]
fn computed_members_and_parenthesized_member_calls_generate_valid_javascript() {
  for expr in [
    "props.items[props.index][props.method]()",
    "(props.service.label)()",
  ] {
    let code = method_code(&format!(
      "export function A(props){{return <p>{{{expr}}}</p>}}"
    ));
    assert!(code.contains("MethodPathWatcher$jg$(props"), "{code}");
  }
}
#[test]
fn optional_member_and_optional_call_keep_short_circuit_calls() {
  for expr in [
    "props.service?.label()",
    "props.service.label?.()",
    "props.service?.label?.()",
  ] {
    let code = method_code(&format!(
      "export function A(props){{return <p>{{{expr}}}</p>}}"
    ));
    assert!(code.contains("MethodPathWatcher$jg$(props"), "{code}");
    assert!(code.contains("$jg$a0?.("), "{code}");
  }
}
#[test]
fn function_values_are_not_implicitly_bound() {
  let code =
    method_code("export function A(props){return <button on:click={props.handler}>Click</button>}");
  assert!(!code.contains("MethodPathWatcher$jg$("), "{code}");
}

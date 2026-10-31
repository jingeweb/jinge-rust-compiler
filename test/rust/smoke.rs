use super::super::*;
#[test]
fn transform_component() {
  let (code, components, map) = inner_transform(
    "test.tsx".into(),
    2,
    "export function A(){return <p>hello</p>}".into(),
    false,
    IntlType::Disabled,
  )
  .unwrap();
  assert!(code.contains("createEle$jg$"));
  assert_eq!(components, "A");
  assert!(map.is_none());
}
#[test]
fn diagnostics_are_errors() {
  assert!(
    inner_transform(
      "test.tsx".into(),
      2,
      "export function A(){return <div {...attrs}/>}".into(),
      false,
      IntlType::Disabled
    )
    .is_err()
  );
}

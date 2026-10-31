use super::*;
fn transform(
  input: &str,
  code_type: usize,
  intl: IntlType,
) -> Result<(String, String, Option<String>), String> {
  inner_transform("contract.tsx".into(), code_type, input.into(), false, intl)
}
#[test]
fn sourcemap_is_optional_and_uses_the_original_filename() {
  let input = "export function A(){return <p>hi</p>}";
  let (_, _, map) = inner_transform(
    "src/example.tsx".into(),
    2,
    input.into(),
    true,
    IntlType::Disabled,
  )
  .unwrap();
  let map = map.unwrap();
  assert!(map.contains("src/example.tsx"));
  assert!(map.contains("mappings"));
  assert!(transform(input, 2, IntlType::Disabled).unwrap().2.is_none());
}
#[test]
fn errors_do_not_poison_the_next_transform_or_symbol_table() {
  for _ in 0..3 {
    assert!(transform("export function A(){return <div>}", 2, IntlType::Disabled).is_err());
    let (code, names, _) = transform(
      "export function A(){return <p>ok</p>}",
      2,
      IntlType::Disabled,
    )
    .unwrap();
    assert_javascript(&code);
    assert_eq!(names, "A");
  }
}
#[test]
fn ts_generics_and_assertions_are_not_parsed_as_jsx() {
  let (code, names, _) = transform(
    "export const identity=<T>(x:T):T=>x;export const n=<number>123;",
    1,
    IntlType::Disabled,
  )
  .unwrap();
  assert_javascript(&code);
  assert_eq!(names, "");
  assert!(code.contains("identity"));
  assert!(!code.contains("<T>"));
}
#[test]
fn intl_hash_protocol_vectors() {
  for (message, key) in [
    ("hello", "m3HSJL"),
    ("Hello", "NhX4DJ"),
    ("Hello <B>world</B>", "NH9/Bc"),
  ] {
    assert_eq!(parser::intl::calc_intl_key(message, None), key);
  }
  assert_ne!(
    parser::intl::calc_intl_key("hello", None),
    parser::intl::calc_intl_key("hello", Some("a.tsx"))
  );
}
#[test]
fn transforms_are_isolated_across_threads() {
  let threads: Vec<_> = (0..8)
    .map(|i| {
      std::thread::spawn(move || {
        let source = format!("export function C{i}(){{return <p>{i}</p>}}");
        let (code, names, _) = transform(&source, 2, IntlType::Disabled).unwrap();
        assert_eq!(names, format!("C{i}"));
        assert_javascript(&code);
      })
    })
    .collect();
  for thread in threads {
    thread.join().unwrap();
  }
}

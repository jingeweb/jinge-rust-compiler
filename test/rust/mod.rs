//! Direct compiler tests: no Node, native binding loader, jsdom, or runtime.
use super::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
mod smoke;

fn fixtures_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("test/rust/fixtures")
}
fn option<'a>(options: &'a str, key: &str) -> Option<&'a str> {
  options
    .lines()
    .filter_map(|line| line.split_once('='))
    .find_map(|(k, v)| (k == key).then_some(v))
}
fn assert_javascript(code: &str) {
  let cm = SourceMap::default();
  let source = cm.new_source_file(
    FileName::Custom("generated.js".into()).into(),
    code.to_owned(),
  );
  let lexer = Lexer::new(
    Syntax::Es(Default::default()),
    EsVersion::latest(),
    SourceFileInput::from(&*source),
    None,
  );
  let mut parser = Parser::new_from(lexer);
  assert!(
    parser.parse_program().is_ok(),
    "generated output is not JavaScript:\n{code}"
  );
  assert!(
    parser.take_errors().is_empty(),
    "generated output contains parser errors:\n{code}"
  );
}
fn run_fixture(name: &str) {
  let dir = fixtures_dir().join(name);
  let input = std::fs::read_to_string(dir.join("input.tsx.txt")).unwrap();
  let options = std::fs::read_to_string(dir.join("options.txt")).unwrap();
  let code_type = option(&options, "type").unwrap_or("2").parse().unwrap();
  let intl = match option(&options, "intl").unwrap_or("0") {
    "0" => IntlType::Disabled,
    "1" => IntlType::Enabled(false),
    "2" => IntlType::Enabled(true),
    other => panic!("invalid intl mode {other}"),
  };
  let result = inner_transform(
    if code_type == 1 {
      "fixture.ts"
    } else {
      "fixture.tsx"
    }
    .into(),
    code_type,
    input,
    false,
    intl,
  );
  if dir.join("expected.error").exists() {
    let error = result.expect_err("unsupported input must fail compilation");
    let expected = std::fs::read_to_string(dir.join("expected.error")).unwrap();
    assert_eq!(error, expected, "diagnostic changed for {name}");
    assert!(!error.contains("internal error"));
    return;
  }
  let (code, components, map) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
  assert_javascript(&code);
  assert_eq!(
    components,
    option(&options, "components").unwrap_or(""),
    "component metadata: {name}"
  );
  assert!(map.is_none());
  let output_path = dir.join("expected.js");
  if std::env::var("UPDATE_RUST_SNAPSHOTS").as_deref() == Ok("1") {
    std::fs::write(&output_path, &code).unwrap();
  }
  let expected = std::fs::read_to_string(&output_path).unwrap();
  assert_eq!(code, expected, "codegen snapshot changed: {name}");
}
macro_rules! fixture_tests {
  ($($test:ident => $fixture:literal),* $(,)?) => {
    const CASES: &[&str] = &[$($fixture),*];
    $(#[test] fn $test() { run_fixture($fixture); })*
    #[test]
    fn all_fixture_directories_are_registered() {
      let actual: BTreeSet<_> = std::fs::read_dir(fixtures_dir()).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
      assert_eq!(actual, CASES.iter().map(|name| name.to_string()).collect());
    }
  };
}
mod codegen;
mod contracts;

mod dom_paths;

mod svg;

mod slot_empty;

mod intl_bindings;

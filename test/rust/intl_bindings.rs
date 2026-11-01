use super::*;
fn compile_intl(source: &str, kind: usize) -> String {
  let (code, _, _) = inner_transform(
    "intl-binding.tsx".into(),
    kind,
    source.into(),
    false,
    IntlType::Enabled(false),
  )
  .unwrap();
  assert_javascript(&code);
  code
}
#[test]
fn direct_import_alias_is_recognized_in_ts_and_tsx() {
  for kind in [1, 2] {
    let code = compile_intl(
      "import {t as tr} from 'jinge';export const label=tr('Hello');",
      kind,
    );
    assert!(code.contains("\"NhX4DJ\""), "{code}");
  }
}
#[test]
fn alias_is_recognized_in_templates_attributes_and_root_returns() {
  for source in [
    "import {t as tr} from 'jinge';export function A(){return <p title={tr('Hello')+'!'}>{tr('Hello')}</p>}",
    "import {t as tr} from 'jinge';export function A(){return tr('Hello')}",
  ] {
    let code = compile_intl(source, 2);
    assert!(code.contains("\"NhX4DJ\""), "{code}");
    assert!(code.contains("renderIntlText$jg$("), "{code}");
  }
}
#[test]
fn local_t_functions_are_not_transformed() {
  for kind in [1, 2] {
    let code = compile_intl(
      "function t(value){return '['+value+']'}export const label=t('Hello');",
      kind,
    );
    assert!(!code.contains("NhX4DJ"), "{code}");
  }
  let code = compile_intl(
    "function t(value){return '['+value+']'}export function A(){return <p>{t('Hello')}</p>}",
    2,
  );
  assert!(!code.contains("NhX4DJ"), "{code}");
}
#[test]
fn imported_bindings_can_be_shadowed() {
  for name in ["t", "tr"] {
    let code = compile_intl(
      &format!(
        "import {{t as {name}}} from 'jinge';export function local({name}){{return {name}('Hello')}}export const label={name}('Hello');"
      ),
      1,
    );
    assert_eq!(code.matches("\"NhX4DJ\"").count(), 1, "{code}");
  }
}
#[test]
fn indirect_imports_namespace_and_secondary_aliases_stay_ordinary_calls() {
  for source in [
    "import {t} from './a';export const label=t('Hello');",
    "import * as jinge from 'jinge';export const label=jinge.t('Hello');",
    "import {t} from 'jinge';const tr=t;export const label=tr('Hello');",
    "import type {t} from 'jinge';export const label=t('Hello');",
  ] {
    for kind in [1, 2] {
      let code = compile_intl(source, kind);
      assert!(!code.contains("NhX4DJ"), "{code}");
    }
  }
}
#[test]
fn import_order_does_not_affect_recognition() {
  let code = compile_intl(
    "export const label=tr('Hello');import {t as tr} from 'jinge';",
    1,
  );
  assert!(code.contains("\"NhX4DJ\""), "{code}");
}

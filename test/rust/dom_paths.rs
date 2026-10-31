use super::*;
fn compile_dom(source: &str) -> String {
  let (code, _, _) = inner_transform(
    "dom-paths.tsx".into(),
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
fn multiple_constant_is_initialized_before_options_are_created() {
  let code = compile_dom(
    "export function A(){return <select multiple><option value='a' selected>A</option><option value='b' selected>B</option></select>}",
  );
  assert!(
    code.find("$jg$.multiple = true").unwrap() < code.find("createEleA$jg$(\"option\"").unwrap(),
    "multiple must be initialized before child rendering:\n{code}"
  );
}
#[test]
fn multiple_watcher_is_initialized_before_options_are_created() {
  let code = compile_dom(
    "import {vm} from 'jinge';const state=vm({multi:true});export function A(){return <select multiple={state.multi}><option value='a' selected>A</option><option value='b' selected>B</option></select>}",
  );
  assert!(
    code.find("watchPathForRender$jg$(state").unwrap()
      < code.find("createEleA$jg$(\"option\"").unwrap(),
    "initial multiple watcher value must precede children:\n{code}"
  );
}
#[test]
fn select_value_is_initialized_after_options() {
  let code = compile_dom(
    "export function A(){return <select value='b'><option value='a'>A</option><option value='b'>B</option></select>}",
  );
  assert!(
    code.rfind("$jg$.value = 'b'").unwrap() > code.rfind("createEleA$jg$(\"option\"").unwrap(),
    "select.value needs existing options:\n{code}"
  );
}
#[test]
fn primitive_members_keep_the_original_reactive_root_and_property_paths() {
  let code = compile_dom(
    "import {vm} from 'jinge';const state=vm({name:'jinge',index:0});export function A(){return <p>{state.name.length}:{state.name[0]}:{state.name[state.index]}</p>}",
  );
  assert!(code.contains("watchPathForRender$jg$(state"));
  assert!(code.contains("DymPathWatcher$jg$(state"));
  assert!(code.contains("\"name\",\n"));
  assert!(code.contains("\"length\""));
  assert!(
    !code.contains("watchPathForRender$jg$(state.name"),
    "a primitive value must not replace the vm root"
  );
}

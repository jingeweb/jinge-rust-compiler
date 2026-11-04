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

#[test]
fn hidden_constants_use_the_native_property() {
  let code = compile_dom("export function A(){return <><div hidden/><div hidden={false}/></>}");
  assert!(code.contains("$jg$.hidden = true"), "{code}");
  assert!(code.contains("$jg$.hidden = false"), "{code}");
}
#[test]
fn hidden_watchers_assign_the_native_property() {
  let code = compile_dom("export function A(props){return <div hidden={props.hidden}/>}");
  assert!(code.contains("$jg$.hidden = v"), "{code}");
}

#[test]
fn inert_constants_use_the_native_property() {
  let code = compile_dom("export function A(){return <><div inert/><div inert={false}/></>}");
  assert!(code.contains("$jg$.inert = true"), "{code}");
  assert!(code.contains("$jg$.inert = false"), "{code}");
}
#[test]
fn inert_watchers_assign_the_native_property() {
  let code = compile_dom("export function A(props){return <div inert={props.inert}/>}");
  assert!(code.contains("$jg$.inert = v"), "{code}");
}

#[test]
fn f07_static_idl_names_lowercase_only_flagged_properties() {
  let code = compile_dom(
    "export function A(){return <><input autoComplete='username' autoFocus readOnly/><video autoPlay/></>}",
  );
  for expected in [
    "$jg$.autocomplete = 'username'",
    "$jg$.autofocus = true",
    "$jg$.readOnly = true",
    "$jg$.autoplay = true",
  ] {
    assert!(code.contains(expected), "{expected}:\n{code}");
  }
}
#[test]
fn f07_reactive_idl_names_lowercase_only_flagged_properties() {
  let code = compile_dom(
    "export function A(props){return <><input autoComplete={props.complete} autoFocus={props.focus} readOnly={props.readOnly}/><video autoPlay={props.play}/></>}",
  );
  for expected in [
    "$jg$.autocomplete = v",
    "$jg$.autofocus = v",
    "$jg$.readOnly = v",
    "$jg$.autoplay = v",
  ] {
    assert!(code.contains(expected), "{expected}:\n{code}");
  }
}

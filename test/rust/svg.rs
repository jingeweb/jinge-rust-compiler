use super::*;

fn compile_svg(source: &str) -> String {
  let (code, _, _) = inner_transform(
    "svg.tsx".into(),
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
fn svg_exclusive_tags_are_recognized_without_a_parent_svg() {
  for tag in [
    "svg",
    "animate",
    "animateMotion",
    "animateTransform",
    "circle",
    "clipPath",
    "defs",
    "desc",
    "ellipse",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "filter",
    "foreignObject",
    "g",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "set",
    "stop",
    "switch",
    "symbol",
    "text",
    "textPath",
    "tspan",
    "use",
    "view",
  ] {
    let code = compile_svg(&format!("export function A(){{return <{tag}/>}}"));
    assert!(
      code.contains(&format!("createSVGEle$jg$(\"{tag}\"")),
      "{tag}:\n{code}"
    );
  }
}

#[test]
fn svg_exclusive_tags_with_attributes_use_svg_creation() {
  let code = compile_svg("export function A(){return <path d='M0 0L10 10'/>}");
  assert!(code.contains("createSVGEleA$jg$(\"path\""), "{code}");
}

#[test]
fn standalone_svg_container_inherits_namespace_but_foreign_object_resets_it() {
  let code = compile_svg(
    "export function A(){return <g><title>SVG title</title><foreignObject><a>HTML link</a><svg><circle/></svg></foreignObject></g>}",
  );
  for tag in ["g", "title", "foreignObject", "svg", "circle"] {
    assert!(
      code.contains(&format!("createSVGEle$jg$(\"{tag}\"")),
      "{tag}:\n{code}"
    );
  }
  assert!(code.contains("createEle$jg$(\"a\""), "{code}");
}

#[test]
fn ambiguous_and_custom_tags_keep_html_default() {
  for tag in [
    "a",
    "title",
    "style",
    "script",
    "div",
    "my-widget",
    "circle-widget",
  ] {
    let code = compile_svg(&format!("export function A(){{return <{tag}/>}}"));
    assert!(
      code.contains(&format!("createEle$jg$(\"{tag}\"")),
      "{tag}:\n{code}"
    );
  }
}

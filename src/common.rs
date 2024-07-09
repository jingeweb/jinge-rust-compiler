macro_rules! x {
  ($name:literal) => {
    ($name, concat!("jinge$", $name, "$"))
  };
}

// TODO: should use macro to generate

pub const JINGE_IMPORT_TEXT_RENDER_FN: (&str, &str) = x!("textRenderFn");
pub const JINGE_IMPORT_CREATE_ELE: (&str, &str) = x!("createEle");
pub const JINGE_IMPORT_CREATE_ELE_A: (&str, &str) = x!("createEleA");

pub const JINGE_IMPORTS: [(&str, &str); 3] = [
  JINGE_IMPORT_TEXT_RENDER_FN,
  JINGE_IMPORT_CREATE_ELE,
  JINGE_IMPORT_CREATE_ELE_A,
];

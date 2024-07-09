use enumset::EnumSetType;
use strum_macros::AsRefStr;

#[derive(EnumSetType, AsRefStr)]
pub enum ImportId {
  #[strum(serialize = "textRenderFn")]
  TextRenderFn,
}

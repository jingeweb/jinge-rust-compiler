use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub delete_default_message: Option<bool>,
}

use std::fmt::Display;
use std::{error, fmt};

#[derive(Debug)]
pub struct VariantNotFound {
  pub(crate) v: String,
}

impl error::Error for VariantNotFound {}

impl Display for VariantNotFound {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "VariantNotFound: {}", &self.v)
  }
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum ValheimModError {
  #[error("Invalid Valheim mod URL")]
  InvalidUrl,
  #[error("Failed to download the mod! Check logs!")]
  DownloadFailed,
}

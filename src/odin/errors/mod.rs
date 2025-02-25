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
  #[error("Failed to download the mod! Check logs!")]
  DownloadFailed,
  #[error("Invalid Valheim mod URL")]
  InvalidUrl,
  #[error("Directory creation error: {0}")]
  DirectoryCreationError(String),
  #[error("Extraction error: {0}")]
  ExtractionError(String),
  #[error("Invalid staging location")]
  InvalidStagingLocation,
  #[error("File open error: {0}")]
  FileOpenError(String),
  #[error("Zip archive error: {0}")]
  ZipArchiveError(String),
  #[error("Download error: {0}")]
  DownloadError(String),
  #[error("File creation error: {0}")]
  FileCreateError(String),
  #[error("File move error: {0}")]
  FileMoveError(String),
  #[error("Temporary directory creation error: {0}")]
  TempDirCreationError(String),
  #[error("Failed to deserialize manifest file: {0}")]
  ManifestDeserializeError(String),
}

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
  #[error("File copy error: {0}")]
  FileCopyError(String),
  #[error("Directory creation error: {0}")]
  DirectoryCreationError(String),
  #[error("Extraction error: {0}")]
  ExtractionError(String),
  #[error("Invalid staging location")]
  InvalidStagingLocation,
  #[error("File name error: {0}")]
  FileNameError(String),
  #[error("File rename error: {0}")]
  FileRenameError(String),
  #[error("File open error: {0}")]
  FileOpenError(String),
  #[error("Zip archive error: {0}")]
  ZipArchiveError(String),
  #[error("Directory not found: {0}")]
  DirectoryNotFound(String),
  #[error("Download error: {0}")]
  DownloadError(String),
  #[error("File creation error: {0}")]
  FileCreateError(String),
}

use crate::errors::ValheimModError::ManifestDeserializeError;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Manifest {
  pub name: String,
  pub dependencies: Option<Vec<String>>,
  pub version_number: Option<String>,
}

impl TryFrom<PathBuf> for Manifest {
  type Error = Box<dyn std::error::Error>;

  fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
    debug!("Reading Manifest from {:?}", &value);

    if !value.exists() {
      return Err(Box::new(ManifestDeserializeError(format!(
        "Failed to find manifest at {:?}",
        &value
      ))));
    }

    let mut file = File::open(&value)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    if content.trim().is_empty() {
      return Err(Box::new(ManifestDeserializeError(format!(
        "Manifest file at {:?} is empty",
        &value
      ))));
    }

    // Check for BOM and remove it if present
    if content.starts_with('\u{FEFF}') {
      content = content.trim_start_matches('\u{FEFF}').to_string();
    }

    debug!("Manifest content: {}", content);

    let manifest: Manifest = serde_json::from_str(&content)
      .map_err(|e| ManifestDeserializeError(format!("Failed to deserialize manifest: {}", e)))?;

    Ok(manifest)
  }
}

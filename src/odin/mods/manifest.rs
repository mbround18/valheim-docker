use crate::errors::ValheimModError::ManifestDeserializeError;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;
  use tempfile::tempdir;

  #[test]
  fn test_deserialize_manifest_basic() {
    let json = r#"{"name":"TestMod","version_number":"1.0.0"}"#;
    let m: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(m.name, "TestMod");
    assert_eq!(m.version_number, Some("1.0.0".to_string()));
    assert_eq!(m.dependencies, None);
  }

  #[test]
  fn test_deserialize_manifest_with_dependencies() {
    let json = r#"{
      "name":"ModWithDeps",
      "version_number":"2.5.1",
      "dependencies":["Dep1","Dep2"]
    }"#;
    let m: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(m.name, "ModWithDeps");
    assert_eq!(m.version_number, Some("2.5.1".to_string()));
    assert_eq!(
      m.dependencies,
      Some(vec!["Dep1".to_string(), "Dep2".to_string()])
    );
  }

  #[test]
  fn test_deserialize_manifest_without_version() {
    let json = r#"{"name":"NoVersionMod"}"#;
    let m: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(m.name, "NoVersionMod");
    assert_eq!(m.version_number, None);
    assert_eq!(m.dependencies, None);
  }

  #[test]
  fn test_try_from_file_success() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("manifest.json");
    let mut file = File::create(&manifest_path).unwrap();
    writeln!(file, r#"{{"name":"FileMod","version_number":"1.2.3"}}"#).unwrap();

    let result = Manifest::try_from(manifest_path);
    assert!(result.is_ok());
    let m = result.unwrap();
    assert_eq!(m.name, "FileMod");
    assert_eq!(m.version_number, Some("1.2.3".to_string()));
  }

  #[test]
  fn test_try_from_file_not_found() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("nonexistent.json");
    let result = Manifest::try_from(manifest_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_try_from_file_empty() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("manifest.json");
    File::create(&manifest_path).unwrap();

    let result = Manifest::try_from(manifest_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_try_from_file_invalid_json() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("manifest.json");
    let mut file = File::create(&manifest_path).unwrap();
    writeln!(file, "{{invalid json").unwrap();

    let result = Manifest::try_from(manifest_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_try_from_file_with_bom() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("manifest.json");
    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(b"\xef\xbb\xbf").unwrap();
    writeln!(file, r#"{{"name":"BOMTest","version_number":"1.0.0"}}"#).unwrap();

    let result = Manifest::try_from(manifest_path);
    assert!(result.is_ok());
    let m = result.unwrap();
    assert_eq!(m.name, "BOMTest");
  }

  #[test]
  fn test_try_from_file_with_whitespace() {
    let tmp = tempdir().unwrap();
    let manifest_path = tmp.path().join("manifest.json");
    let mut file = File::create(&manifest_path).unwrap();
    writeln!(file, "  ").unwrap();

    let result = Manifest::try_from(manifest_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_serialize_manifest() {
    let m = Manifest {
      name: "TestMod".to_string(),
      version_number: Some("1.5.0".to_string()),
      dependencies: Some(vec!["DepA".to_string(), "DepB".to_string()]),
    };
    let json = serde_json::to_string(&m).unwrap();
    let m2: Manifest = serde_json::from_str(&json).unwrap();
    assert_eq!(m, m2);
  }

  #[test]
  fn test_manifest_equality() {
    let m1 = Manifest {
      name: "Mod".to_string(),
      version_number: Some("1.0.0".to_string()),
      dependencies: None,
    };
    let m2 = Manifest {
      name: "Mod".to_string(),
      version_number: Some("1.0.0".to_string()),
      dependencies: None,
    };
    assert_eq!(m1, m2);
  }
}

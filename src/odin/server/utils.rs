use super::process::ServerProcess;
use crate::{constants, utils::get_working_dir};
use std::{fs, path::Path};

pub fn is_running() -> bool {
  ServerProcess::new().are_process_running()
}

/// Attempts to read the current build id from the local Steam app manifest.
/// Returns None if the manifest isn't present yet (e.g., first install).
pub fn try_get_current_build_id() -> Option<String> {
  let manifest_path = Path::new(&get_working_dir())
    .join("steamapps")
    .join(format!("appmanifest_{}.acf", constants::GAME_ID));
  match fs::read_to_string(&manifest_path) {
    Ok(contents) => {
      // Reuse parser from update module
      let id = super::update::extract_build_id_from_manifest(&contents).to_string();
      Some(id)
    }
    Err(_) => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use once_cell::sync::Lazy;
  use serial_test::serial;
  use std::env;
  use std::fs;
  use std::path::{Path, PathBuf};
  use tempfile::TempDir;

  static TEST_ASSET_DIR: Lazy<PathBuf> = Lazy::new(|| {
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("tests")
      .join("assets")
  });

  #[test]
  #[serial]
  fn returns_none_when_manifest_missing() {
    let tmp = TempDir::new().unwrap();
    env::set_var(crate::constants::ODIN_WORKING_DIR, tmp.path());

    // Ensure no file exists
    assert!(try_get_current_build_id().is_none());
  }

  #[test]
  #[serial]
  fn returns_build_id_when_manifest_present() {
    let tmp = TempDir::new().unwrap();
    env::set_var(crate::constants::ODIN_WORKING_DIR, tmp.path());

    let manifest_contents =
      fs::read_to_string(TEST_ASSET_DIR.join("example_current_app_manifest.txt")).unwrap();

    let manifest_path = tmp
      .path()
      .join("steamapps")
      .join(format!("appmanifest_{}.acf", constants::GAME_ID));
    fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
    fs::write(&manifest_path, manifest_contents).unwrap();

    let id = try_get_current_build_id();
    assert!(id.is_some());
    assert_eq!(id.unwrap(), "6246034");
  }
}

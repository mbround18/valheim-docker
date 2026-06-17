use crate::constants::MODS_LOCATION;
use crate::mods::manifest::Manifest;
use crate::utils::common_paths::bepinex_plugin_directory;
use glob::glob;
use log::{debug, error};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub struct InstalledMod {
  pub manifest: Manifest,
  pub path: String,
}

pub fn installed_mods_with_paths() -> Vec<InstalledMod> {
  // Retrieve the MODS_LOCATION environment variable
  let mods_location = match env::var(MODS_LOCATION) {
    Ok(path) => path,
    Err(e) => {
      error!("Failed to read MODS_LOCATION environment variable: {}", e);
      return vec![];
    }
  };

  // Construct the glob pattern to find all manifest.json files
  let pattern = format!("{}/**/manifest.json", mods_location);

  // Use glob to find files matching the pattern
  let paths = match glob(&pattern) {
    Ok(paths) => paths,
    Err(e) => {
      error!("Failed to read glob pattern {}: {}", pattern, e);
      return vec![];
    }
  };

  let mut results = Vec::new();
  let mut found_mods = HashMap::new();

  for entry in paths {
    match entry {
      Ok(path) => {
        debug!("Found manifest file: {}", path.display());
        match Manifest::try_from(PathBuf::from(&path)) {
          Ok(manifest) => {
            found_mods.insert(manifest.name.clone(), true);
            results.push(InstalledMod {
              manifest,
              path: path.to_string_lossy().into(),
            });
          }
          Err(e) => error!("Failed to deserialize JSON from {}: {}", path.display(), e),
        }
      }
      Err(e) => error!("Error reading path: {}", e),
    }
  }

  // Only add DLL-discovered mods if we haven't already found them
  for dll_mod in discover_plugin_dlls() {
    if !found_mods.contains_key(&dll_mod.manifest.name) {
      found_mods.insert(dll_mod.manifest.name.clone(), true);
      results.push(dll_mod);
    }
  }

  results
}

fn discover_plugin_dlls() -> Vec<InstalledMod> {
  let plugin_dir = bepinex_plugin_directory();
  let pattern = format!("{}/**/*.dll", plugin_dir);

  let paths = match glob(&pattern) {
    Ok(paths) => paths,
    Err(e) => {
      error!("Failed to read glob pattern {}: {}", pattern, e);
      return vec![];
    }
  };

  let mut results = Vec::new();
  for entry in paths {
    match entry {
      Ok(path) => {
        debug!("Found plugin dll: {}", path.display());
        let manifest = find_manifest_near_plugin(&path).unwrap_or_else(|| manifest_from_dll(&path));
        results.push(InstalledMod {
          manifest,
          path: path.to_string_lossy().into(),
        });
      }
      Err(e) => error!("Error reading path: {}", e),
    }
  }

  results
}

fn find_manifest_near_plugin(dll_path: &Path) -> Option<Manifest> {
  let mut dir_opt = dll_path.parent();
  for _ in 0..2 {
    if let Some(dir) = dir_opt {
      let candidate = dir.join("manifest.json");
      if candidate.exists() {
        return Manifest::try_from(candidate).ok();
      }
      if dir.file_name().map(|n| n == "plugins").unwrap_or(false) {
        break;
      }
      dir_opt = dir.parent();
    }
  }
  None
}

fn manifest_from_dll(dll_path: &Path) -> Manifest {
  let name = dll_path
    .file_stem()
    .and_then(|s| s.to_str())
    .map(|s| s.to_string())
    .unwrap_or_else(|| dll_path.to_string_lossy().into_owned());

  Manifest {
    name,
    dependencies: None,
    version_number: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::fs;
  use std::io::Write;
  use tempfile::tempdir;

  #[test]
  fn test_manifest_from_dll() {
    let path = PathBuf::from("/path/to/CoolMod.dll");
    let manifest = manifest_from_dll(&path);
    assert_eq!(manifest.name, "CoolMod");
    assert_eq!(manifest.version_number, None);
    assert_eq!(manifest.dependencies, None);
  }

  #[test]
  fn test_manifest_from_dll_with_extension() {
    let path = PathBuf::from("/plugins/MyPlugin.dll");
    let manifest = manifest_from_dll(&path);
    assert_eq!(manifest.name, "MyPlugin");
  }

  #[test]
  fn test_find_manifest_near_plugin_in_same_dir() {
    let tmp = tempdir().unwrap();
    let dll_path = tmp.path().join("TestMod.dll");
    let manifest_path = tmp.path().join("manifest.json");

    fs::write(&dll_path, "").unwrap();
    let mut manifest_file = fs::File::create(&manifest_path).unwrap();
    writeln!(
      manifest_file,
      r#"{{"name":"TestMod","version_number":"1.0.0"}}"#
    )
    .unwrap();

    let result = find_manifest_near_plugin(&dll_path);
    assert!(result.is_some());
    let manifest = result.unwrap();
    assert_eq!(manifest.name, "TestMod");
    assert_eq!(manifest.version_number, Some("1.0.0".to_string()));
  }

  #[test]
  fn test_find_manifest_near_plugin_in_parent_dir() {
    let tmp = tempdir().unwrap();
    let subdir = tmp.path().join("TestMod");
    fs::create_dir_all(&subdir).unwrap();

    let dll_path = subdir.join("plugin.dll");
    let manifest_path = subdir.join("manifest.json");

    fs::write(&dll_path, "").unwrap();
    let mut manifest_file = fs::File::create(&manifest_path).unwrap();
    writeln!(
      manifest_file,
      r#"{{"name":"TestMod","version_number":"2.5.0"}}"#
    )
    .unwrap();

    let result = find_manifest_near_plugin(&dll_path);
    assert!(result.is_some());
    let manifest = result.unwrap();
    assert_eq!(manifest.name, "TestMod");
  }

  #[test]
  fn test_find_manifest_near_plugin_not_found() {
    let tmp = tempdir().unwrap();
    let dll_path = tmp.path().join("NoManifestMod.dll");
    fs::write(&dll_path, "").unwrap();

    let result = find_manifest_near_plugin(&dll_path);
    assert!(result.is_none());
  }

  #[test]
  fn test_find_manifest_respects_depth_limit() {
    let tmp = tempdir().unwrap();
    let deep_dir = tmp.path().join("a").join("b").join("c");
    fs::create_dir_all(&deep_dir).unwrap();

    let dll_path = deep_dir.join("deep.dll");
    fs::write(&dll_path, "").unwrap();

    let manifest_path = tmp.path().join("manifest.json");
    let mut manifest_file = fs::File::create(&manifest_path).unwrap();
    writeln!(
      manifest_file,
      r#"{{"name":"Deep","version_number":"1.0.0"}}"#
    )
    .unwrap();

    let result = find_manifest_near_plugin(&dll_path);
    assert!(result.is_none());
  }

  #[test]
  fn test_find_manifest_stops_at_plugins_root() {
    let tmp = tempdir().unwrap();
    let plugins_dir = tmp.path().join("plugins");
    let mod_dir = plugins_dir.join("TestMod");
    fs::create_dir_all(&mod_dir).unwrap();

    let dll_path = mod_dir.join("plugin.dll");
    fs::write(&dll_path, "").unwrap();

    let parent_of_plugins = tmp.path().join("manifest.json");
    let mut manifest_file = fs::File::create(&parent_of_plugins).unwrap();
    writeln!(
      manifest_file,
      r#"{{"name":"ShouldStop","version_number":"1.0.0"}}"#
    )
    .unwrap();

    let result = find_manifest_near_plugin(&dll_path);
    assert!(
      result.is_none(),
      "Should not find manifest above plugins directory"
    );
  }

  #[test]
  fn test_installed_mods_with_paths_empty_when_env_var_missing() {
    let _ = env::var(MODS_LOCATION);
    env::remove_var(MODS_LOCATION);
    let result = installed_mods_with_paths();
    assert!(result.is_empty());
  }

  #[test]
  #[serial]
  fn test_installed_mods_deduplication() {
    let original_mods_loc = env::var(MODS_LOCATION).ok();

    let tmp = tempdir().unwrap();
    env::set_var(MODS_LOCATION, tmp.path());

    let mod_dir = tmp.path().join("MyMod");
    fs::create_dir_all(&mod_dir).unwrap();
    let mut manifest_file = fs::File::create(mod_dir.join("manifest.json")).unwrap();
    writeln!(
      manifest_file,
      r#"{{"name":"MyMod","version_number":"1.0.0"}}"#
    )
    .unwrap();

    let result = installed_mods_with_paths();
    let mod_count = result.iter().filter(|m| m.manifest.name == "MyMod").count();
    assert!(mod_count >= 1, "Should find at least one MyMod");
    assert_eq!(mod_count, 1, "Should not have duplicate mods");

    if let Some(loc) = original_mods_loc {
      env::set_var(MODS_LOCATION, loc);
    } else {
      env::remove_var(MODS_LOCATION);
    }
  }

  #[test]
  fn test_installed_mod_struct_fields() {
    let manifest = Manifest {
      name: "TestMod".to_string(),
      version_number: Some("1.0.0".to_string()),
      dependencies: None,
    };
    let installed_mod = InstalledMod {
      manifest,
      path: "/path/to/manifest.json".to_string(),
    };
    assert_eq!(installed_mod.manifest.name, "TestMod");
    assert_eq!(installed_mod.path, "/path/to/manifest.json");
  }

  #[test]
  fn test_manifest_from_dll_preserves_case() {
    let path = PathBuf::from("/plugins/MyCoolMod.dll");
    let manifest = manifest_from_dll(&path);
    assert_eq!(manifest.name, "MyCoolMod");
  }

  #[test]
  fn test_manifest_from_dll_various_extensions() {
    let test_cases = vec![
      ("/path/Mod.dll", "Mod"),
      ("/path/Plugin.DLL", "Plugin"),
      ("/path/Framework.framework.dll", "Framework.framework"),
    ];

    for (path, expected_name) in test_cases {
      let manifest = manifest_from_dll(&PathBuf::from(path));
      assert_eq!(manifest.name, expected_name, "Failed for path: {}", path);
    }
  }
}

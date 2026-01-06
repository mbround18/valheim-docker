use crate::constants::MODS_LOCATION;
use crate::mods::manifest::Manifest;
use crate::utils::common_paths::bepinex_plugin_directory;
use glob::glob;
use log::{debug, error};
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
  for entry in paths {
    match entry {
      Ok(path) => {
        debug!("Found manifest file: {}", path.display());
        match Manifest::try_from(PathBuf::from(&path)) {
          Ok(manifest) => results.push(InstalledMod {
            manifest,
            path: path.to_string_lossy().into(),
          }),
          Err(e) => error!("Failed to deserialize JSON from {}: {}", path.display(), e),
        }
      }
      Err(e) => error!("Error reading path: {}", e),
    }
  }

  results.extend(discover_plugin_dlls());

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

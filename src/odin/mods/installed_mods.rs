use crate::constants::MODS_LOCATION;
use crate::mods::manifest::Manifest;
use glob::glob;
use log::{debug, error};
use std::env;
use std::path::PathBuf;

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

  results
}

use crate::constants::MODS_LOCATION;
use crate::mods::manifest::Manifest;
use glob::glob;
use log::{debug, error};
use serde_json::from_reader;
use std::env;
use std::fs::File;
use std::io::BufReader;

pub fn installed_mods() -> Vec<Manifest> {
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

  // Iterate over the found paths and attempt to deserialize them into Manifest structs
  let mut manifests = Vec::new();
  for entry in paths {
    match entry {
      Ok(path) => {
        debug!("Found manifest file: {}", path.display());
        match File::open(&path) {
          Ok(file) => {
            let reader = BufReader::new(file);
            match from_reader(reader) {
              Ok(manifest) => manifests.push(manifest),
              Err(e) => error!("Failed to deserialize JSON from {}: {}", path.display(), e),
            }
          }
          Err(e) => error!("Failed to open file {}: {}", path.display(), e),
        }
      }
      Err(e) => error!("Error reading path: {}", e),
    }
  }

  manifests
}

use crate::errors::ValheimModError;
use crate::mods::manifest::Manifest;
use crate::utils::normalize_paths::normalize_paths;
use crate::utils::{is_valid_url, parse_mod_string};
use crate::{
  constants::SUPPORTED_FILE_TYPES,
  utils::{common_paths, get_md5_hash, parse_file_name, url_parse_file_type},
};
use fs_extra::dir;
use fs_extra::dir::CopyOptions;
use log::{debug, error, info};
use reqwest::Url;
use std::convert::TryFrom;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::ZipArchive;

pub struct ValheimMod {
  pub(crate) url: String,
  pub(crate) file_type: String,
  /// For download, this is the location of the downloaded ZIP.
  pub(crate) staging_location: PathBuf,
  pub(crate) installed: bool,
  pub(crate) downloaded: bool,
  // Optionally, add fields like author or mod_name if needed later.
}

impl ValheimMod {
  pub fn new(url: &str) -> Self {
    let file_type = url_parse_file_type(url);
    ValheimMod {
      url: url.to_string(),
      file_type,
      staging_location: common_paths::mods_staging_directory().into(),
      installed: false,
      downloaded: false,
    }
  }

  /// Determines whether the mod is a framework by inspecting the extracted files.
  fn is_mod_framework(&self, extract_path: &Path) -> bool {
    debug!("Checking mod if it is a framework like bepinex");
    match Manifest::try_from(extract_path.join("manifest.json")) {
      Ok(manifest) => {
        debug!("Parsed manifest with name: {}", manifest.name);
        manifest.name.to_lowercase().starts_with("bepinex")
      }
      Err(_) => {
        for entry in WalkDir::new(extract_path).into_iter().flatten() {
          if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("winhttp.dll")
          {
            return true;
          }
        }
        false
      }
    }
  }

  /// Download: Downloads the mod ZIP from the URL into the staging location.
  pub fn download(&mut self) -> Result<(), ValheimModError> {
    debug!("Initializing mod download...");
    if !self.staging_location.exists() {
      create_dir_all(&self.staging_location).unwrap();
    }

    let parsed_url = Url::parse(&self.url).map_err(|_| ValheimModError::InvalidUrl)?;
    let mut response = reqwest::blocking::get(parsed_url)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

    if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
      debug!("Using redirect URL: {}", &self.url);
      self.url = response.url().to_string();
      self.file_type = url_parse_file_type(response.url().as_ref());
    }

    let file_name = parse_file_name(
      &Url::parse(&self.url).unwrap(),
      &format!("{}.{}", get_md5_hash(&self.url), &self.file_type),
    );
    self.staging_location = self.staging_location.join(file_name);
    debug!("Downloading to: {:?}", self.staging_location);

    let mut file = File::create(&self.staging_location)
      .map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
    response
      .copy_to(&mut file)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
    self.downloaded = true;
    debug!("Download complete: {}", &self.url);
    debug!("Download output: {:?}", self.staging_location);
    Ok(())
  }

  /// Install: Creates a temporary directory, extracts the ZIP there, validates the mod,
  /// moves extracted files to their final destination, and cleans up the temp directory.
  pub fn install(&mut self) -> Result<(), ValheimModError> {
    // Ensure that the staging location is a file (the downloaded ZIP).
    if self.staging_location.is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory: {:?}",
        self.staging_location
      );
      return Err(ValheimModError::InvalidStagingLocation);
    }

    // Create a temporary directory for extraction.
    let temp_dir = tempdir().map_err(|e| {
      ValheimModError::TempDirCreationError(format!("Failed to create temp dir: {}", e))
    })?;
    debug!("Created temporary directory at {:?}", temp_dir.path());

    // Extract the ZIP file (from staging) into the temporary directory.
    {
      let zip_file = File::open(&self.staging_location)
        .map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
      let mut archive =
        ZipArchive::new(zip_file).map_err(|e| ValheimModError::ZipArchiveError(e.to_string()))?;
      archive.extract(temp_dir.path()).map_err(|e| {
        error!("Failed to extract archive: {}", e);
        ValheimModError::ExtractionError(e.to_string())
      })?;

      normalize_paths(temp_dir.path())
        .map_err(|e| ValheimModError::ExtractionError(e.to_string()))?;
    }
    debug!("Extraction complete to {:?}", temp_dir.path());

    // Validate mod type by inspecting the extracted files.
    let is_framework = self.is_mod_framework(temp_dir.path());

    let mut options = CopyOptions {
      overwrite: true,
      skip_exist: false,
      buffer_size: 0,
      copy_inside: false,
      content_only: true,
      depth: 0,
    };

    let manifest = Manifest::try_from(temp_dir.path().join("manifest.json"))
      .map_err(|e| ValheimModError::ManifestDeserializeError(format!("Ayyre buddy {}", e)))?;

    // Move extracted files to the appropriate final destination.
    if is_framework {
      info!("Installing Framework...");
      let final_dir = PathBuf::from(&common_paths::game_directory());
      dir::move_dir(temp_dir.path().join(&manifest.name), &final_dir, &options)
        .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
    } else {
      info!("Installing Mod...");
      let final_dir = PathBuf::from(&common_paths::bepinex_plugin_directory()).join(&manifest.name);
      // If a manifest exists, use its name for a subdirectory.
      create_dir_all(&final_dir)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;

      // Path to the 'plugins' directory within the temp directory
      let plugins_path = temp_dir.path().join("plugins");
      
      if temp_dir.path().join("Plugins").exists() {
        debug!("Looks like someone used Plugins instead of plugins, lets fix that.");
        dir::move_dir(temp_dir.path().join("Plugins"), &plugins_path, &options)
          .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
      }

      // Check if the 'plugins' directory exists
      if plugins_path.exists() && plugins_path.is_dir() {
        let mut plugin_options = options.clone();
        plugin_options.copy_inside = true;
        dir::move_dir(&plugins_path, &final_dir, &plugin_options)
          .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
        // Set options depth of one to maintain manifest.json
        options.depth = 1
      }
      dir::move_dir(temp_dir, &final_dir, &options)
        .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
    }

    // The temporary directory is automatically removed here.
    self.installed = true;
    Ok(())
  }
}

impl TryFrom<String> for ValheimMod {
  type Error = ValheimModError;

  fn try_from(url: String) -> Result<Self, Self::Error> {
    if is_valid_url(&url) {
      Ok(ValheimMod::new(&url))
    } else if let Some((author, mod_name, version)) = parse_mod_string(&url) {
      let constructed_url = format!(
        "https://gcdn.thunderstore.io/live/repository/packages/{}-{}-{}.zip",
        author, mod_name, version
      );
      Ok(ValheimMod::new(&constructed_url))
    } else {
      Err(ValheimModError::InvalidUrl)
    }
  }
}

#[cfg(test)]
mod install_test {
  use super::*;

  // Helper to create a ValheimMod instance with a given staging location.
  fn valheim_mod_with_staging(url: String, staging: PathBuf) -> ValheimMod {
    ValheimMod {
      url,
      staging_location: staging,
      installed: false,
      downloaded: false,
      file_type: "zip".to_string(),
    }
  }

  #[test]
  fn test_install_framework() {
    // Use a test resource ZIP that represents a framework mod.
    let staging = PathBuf::from("tests/resources/manifest.framework.zip");
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);
  }

  #[test]
  fn test_install_mod() {
    // Use a test resource ZIP that represents a regular mod.
    let staging = PathBuf::from("tests/resources/manifest.mod.zip");
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);
  }
}

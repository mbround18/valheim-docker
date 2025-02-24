use crate::errors::ValheimModError;
use crate::mods::manifest::Manifest;
use crate::mods::zipext::ZipExt;
use crate::utils::{is_valid_url, parse_mod_string};
use crate::{
  constants::SUPPORTED_FILE_TYPES,
  utils::{common_paths, get_md5_hash, parse_file_name, url_parse_file_type},
};
use fs_extra::dir::CopyOptions;
use log::{debug, error, info};
use reqwest::Url;
use std::convert::TryFrom;
use std::fs::{self, create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::{result::ZipError, ZipArchive};

pub struct ValheimMod {
  pub(crate) url: String,
  pub(crate) file_type: String,
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
      staging_location: common_paths::mods_directory().into(),
      installed: false,
      downloaded: false,
    }
  }

  fn try_parse_manifest(&self, archive: &mut ZipArchive<File>) -> Result<Manifest, ZipError> {
    debug!("Parsing 'manifest.json' ...");
    let mut manifest_file = archive.by_name("manifest.json")?;
    debug!("'manifest.json' successfully loaded");
    let mut json_data = String::new();
    manifest_file
      .read_to_string(&mut json_data)
      .map_err(|_e| ZipError::FileNotFound)?; // Map I/O error to ZipError (or create a custom error)

    // Remove UTF-8 BOM if present
    let json_data = self.remove_byte_order_mark(json_data);

    serde_json::from_str(&json_data).map_err(|e| {
      error!("Failed to deserialize manifest file: {:?}", e);
      ZipError::FileNotFound
    })
  }

  fn remove_byte_order_mark(&self, value: String) -> String {
    if value.starts_with('\u{feff}') {
      debug!("Found and removed UTF-8 BOM");
      value.trim_start_matches('\u{feff}').to_string()
    } else {
      value
    }
  }

  fn copy_single_file<P1, P2>(&self, from: P1, to: P2) -> Result<(), ValheimModError>
  where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
  {
    let from = from.as_ref();
    let to = to.as_ref();

    let mut options = CopyOptions::new();
    options.overwrite = true;
    fs_extra::copy_items(&[from], to, &options).map_err(|e| {
      error!("Failed to copy file from {:?} to {:?}: {}", from, to, e);
      ValheimModError::FileCopyError(e.to_string())
    })?;
    debug!("Successfully copied {:?} to {:?}", from, to);
    Ok(())
  }

  fn is_mod_framework(&self, archive: &mut ZipArchive<File>) -> bool {
    if let Ok(manifest) = self.try_parse_manifest(archive) {
      let mod_dir = format!("{}/", manifest.name);
      let mod_dir_exists = archive
        .file_names()
        .any(|file_name| file_name.starts_with(&mod_dir));

      debug!("Validating if file is a framework");
      mod_dir_exists
        && (manifest.name == "BepInExPack_Valheim" || manifest.name == "BepInEx_Valheim_Full")
    } else {
      // Fall back to checking for "winhttp.dll"
      archive
        .file_names()
        .any(|file_name| file_name.eq_ignore_ascii_case("winhttp.dll"))
    }
  }

  fn extract_plugin(&self, archive: &mut ZipArchive<File>) -> Result<(), ValheimModError> {
    // Determine output directory and optional sub-directory based on whether it is a framework.
    let (output_dir, archive_dir) = if self.is_mod_framework(archive) {
      info!("Installing Framework...");
      debug!("Zip file is a framework, processing it in parts.");
      let output_dir = PathBuf::from(&common_paths::game_directory());

      let sub_dir = if let Ok(manifest) = self.try_parse_manifest(archive) {
        format!("{}/", manifest.name)
      } else {
        String::new()
      };

      (output_dir, sub_dir)
    } else {
      info!("Installing Mod...");
      let mut output_dir = PathBuf::from(&common_paths::bepinex_plugin_directory());
      if let Ok(manifest) = self.try_parse_manifest(archive) {
        output_dir.push(manifest.name);
      }
      create_dir_all(&output_dir).map_err(|e| {
        error!("Failed to create mod directory {:?}: {}", output_dir, e);
        ValheimModError::DirectoryCreationError(e.to_string())
      })?;

      (output_dir, String::new())
    };

    archive
      .extract_sub_dir_custom(output_dir.clone(), &archive_dir)
      .map_err(|e| {
        error!(
          "Failed to install mod: {}\nDownloaded Archive: {:?}\nError: {}",
          self.url, self.staging_location, e
        );
        ValheimModError::ExtractionError(e.to_string())
      })?;
    info!("Successfully installed {}", &self.url);
    Ok(())
  }

  pub fn install(&mut self) -> Result<(), ValheimModError> {
    if self.staging_location.is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory: {:?}",
        self.staging_location
      );
      return Err(ValheimModError::InvalidStagingLocation);
    }

    if self.file_type == "dll" {
      debug!("Copying downloaded dll to BepInEx plugin directory...");
      self.copy_single_file(
        &self.staging_location,
        common_paths::bepinex_plugin_directory(),
      )?;
    } else if self.file_type == "cfg" {
      debug!("Copying single cfg into config directory");
      let cfg_file_name = self
        .staging_location
        .file_name()
        .ok_or_else(|| ValheimModError::FileNameError("Missing cfg file name".to_string()))?
        .to_owned();

      let mut dst_file_path =
        Path::new(&common_paths::bepinex_config_directory()).join(&cfg_file_name);
      if dst_file_path.exists() {
        dst_file_path = dst_file_path.with_extension("cfg.new");
      }
      fs::rename(&self.staging_location, dst_file_path)
        .map_err(|e| ValheimModError::FileRenameError(e.to_string()))?;
    } else {
      let zip_file = File::open(&self.staging_location)
        .map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
      let mut archive = ZipArchive::new(zip_file).map_err(|e| {
        error!(
          "Failed to parse zip file {:?}: {}",
          self.staging_location, e
        );
        ValheimModError::ZipArchiveError(e.to_string())
      })?;
      self.extract_plugin(&mut archive)?;
    }
    self.installed = true;
    Ok(())
  }

  pub fn download(&mut self) -> Result<(), ValheimModError> {
    debug!("Initializing mod download...");
    if !self.staging_location.exists() {
      error!(
        "Staging location does not exist: {:?}",
        self.staging_location
      );
      return Err(ValheimModError::DirectoryNotFound(format!(
        "Directory does not exist: {:?}",
        self.staging_location
      )));
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
mod zip_test {
  use super::*;
  use std::env::temp_dir;
  use std::path::Path;

  fn load_zip(file: &str) -> ZipArchive<File> {
    ZipArchive::new(File::open(Path::new(file)).unwrap()).unwrap()
  }

  fn valheim_mod(url: String) -> ValheimMod {
    ValheimMod {
      url,
      staging_location: temp_dir(),
      installed: false,
      downloaded: false,
      file_type: "zip".to_string(),
    }
  }

  macro_rules! test_zip {
    ($name:ident, $file:expr, $expected:expr) => {
      #[test]
      fn $name() {
        let mut zip = load_zip($file);
        let info = valheim_mod("test-url".to_string());
        assert_eq!(info.is_mod_framework(&mut zip), $expected);
      }
    };
  }
  test_zip!(
    test_is_mod_framework,
    "tests/resources/manifest.framework.zip",
    true
  );
  test_zip!(
    test_is_not_mod_framework,
    "tests/resources/manifest.mod.zip",
    false
  );

  macro_rules! test_is_this_download_a_framework {
    ($name:ident, $url:expr, $expected:expr) => {
      #[test]
      fn $name() {
        let mut info = valheim_mod($url);
        info.download().unwrap();

        let zip_file = File::open(&info.staging_location).unwrap();
        let mut zip = ZipArchive::new(zip_file).unwrap();
        assert_eq!(info.is_mod_framework(&mut zip), $expected);
      }
    };
  }

  test_is_this_download_a_framework!(
    test_bepinexpack_valheim_v5_4_2102_is_a_framework,
    format!(
      "https://gcdn.thunderstore.io/live/repository/packages/denikson-BepInExPack_Valheim-{}.zip",
      "5.4.2102"
    ),
    true
  );

  test_is_this_download_a_framework!(
    test_bepinexpack_valheim_v5_4_6_is_not_a_framework,
    format!(
      "https://gcdn.thunderstore.io/live/repository/packages/denikson-BepInExPack_Valheim-{}.zip",
      "5.4.6"
    ),
    true
  );
}

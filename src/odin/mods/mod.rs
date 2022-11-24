pub mod bepinex;

use crate::{
  constants::SUPPORTED_FILE_TYPES,
  utils::{common_paths, get_md5_hash, parse_file_name, url_parse_file_type},
};
use fs_extra::dir::CopyOptions;
use log::{debug, error, info};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs::{self, create_dir_all, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::exit;
use zip::{
  result::{ZipError, ZipResult},
  ZipArchive,
};

trait ZipExt {
  fn extract_sub_dir_custom<P: AsRef<Path>>(&mut self, dst_dir: P, sub_dir: &str) -> ZipResult<()>;
}

impl ZipExt for ZipArchive<File> {
  fn extract_sub_dir_custom<P: AsRef<Path>>(&mut self, dst_dir: P, sub_dir: &str) -> ZipResult<()> {
    for i in 0..self.len() {
      let mut file = self.by_index(i)?;
      let filepath = match file
        .enclosed_name()
        .ok_or(ZipError::InvalidArchive("Invalid file path"))?
        .strip_prefix(sub_dir)
      {
        Ok(path) => path,
        Err(_) => continue,
      };

      let mut out_path = dst_dir.as_ref().join(filepath);

      debug!("Extracting file: {:?}", out_path);

      if file.name().ends_with('/') {
        fs::create_dir_all(&out_path)?;
      } else {
        if let Some(p) = out_path.parent() {
          if !p.exists() {
            fs::create_dir_all(p)?;
          }
        }

        // Don't overwrite old cfg files
        if out_path.extension().unwrap_or_default() == "cfg" && out_path.exists() {
          debug!("File is config with already exiting destination! Adding '.new'");
          out_path = out_path.with_extension("cfg.new");
        }

        let mut outfile = File::create(&out_path)?;
        io::copy(&mut file, &mut outfile)?;
        debug!("Extracted file {:?}", out_path);
      }

      // Get and Set permissions
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = file.unix_mode() {
          fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
        }
      }
    }

    Ok(())
  }
}

pub struct ValheimMod {
  pub(crate) url: String,
  pub(crate) file_type: String,
  pub(crate) staging_location: PathBuf,
  pub(crate) installed: bool,
  pub(crate) downloaded: bool,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
  name: String,
}

impl ValheimMod {
  pub fn new(url: &str) -> ValheimMod {
    let file_type = url_parse_file_type(url);
    ValheimMod {
      url: String::from(url),
      file_type,
      staging_location: common_paths::mods_directory().into(),
      installed: false,
      downloaded: false,
    }
  }

  fn try_parse_manifest(&self, archive: &mut ZipArchive<File>) -> Result<Manifest, ZipError> {
    debug!("Parsing 'manifest.json' ...");

    match archive.by_name("manifest.json") {
      Ok(mut manifest) => {
        debug!("'manifest.json' successfully loaded");
        let mut json_data = String::new();
        manifest.read_to_string(&mut json_data).unwrap();

        // Some manifest files include a UTF-8 BOM sequence, breaking serde json parsing
        // See https://github.com/serde-rs/serde/issues/1753
        json_data = self.remove_byte_order_mark(json_data);

        Ok(serde_json::from_str(&json_data).expect("Failed to deserialize manifest file."))
      }
      Err(error) => {
        error!("Failed to deserialize manifest file: {:?}", error);
        Err(error)
      }
    }
  }

  fn remove_byte_order_mark(&self, value: String) -> String {
    if value.contains('\u{feff}') {
      debug!("Found and removed UTF-8 BOM");
      return value.trim_start_matches('\u{feff}').to_string();
    }

    value
  }

  fn copy_single_file<P1, P2>(&self, from: P1, to: P2)
  where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
  {
    let to = to.as_ref();
    let from = from.as_ref();

    let mut dir_options = CopyOptions::new();
    dir_options.overwrite = true;
    match fs_extra::copy_items(&[&from], to, &dir_options) {
      Ok(_) => debug!("Successfully copied {:?} to {:?}", from, to),
      Err(_) => {
        error!("Failed to install {}", self.url);
        error!(
          "File failed to copy from: \n{:?}To Destination:{:?}",
          from, to
        );
        // TODO: Remove Exit Code and provide an Ok or Err.
        exit(1);
      }
    };
  }

  fn is_mod_framework(&self, archive: &mut ZipArchive<File>) -> bool {
    if let Ok(maybe_manifest) = self.try_parse_manifest(archive) {
      let name = maybe_manifest.name;
      let mod_dir = format!("{}/", name);
      let mod_dir_exists = archive.file_names().any(|file_name| file_name == mod_dir);

      // It's a mod framework based on a specific name and if it has a matching directory in the
      // archive
      debug!("Validating if file is a framework");
      mod_dir_exists && (name == "BepInExPack_Valheim" || name == "BepInEx_Valheim_Full")
    } else {
      archive
        // If there is no manifest, fall back to checking for winhttp.dll as a heuristic
        .file_names()
        .any(|file_name| file_name.eq_ignore_ascii_case("winhttp.dll"))
    }
  }

  fn extract_plugin(&self, archive: &mut ZipArchive<File>) {
    // The output location to extract into and the directory to extract from the archive depends on
    // if we're installing just a mod or a full framework, and if it is being downloaded from
    // thunderstore where a manifest is provided, or not.
    let (output_dir, archive_dir) = if self.is_mod_framework(archive) {
      info!("Installing Framework...");
      debug!("Zip file is a framework, processing it in parts.");
      let output_dir = PathBuf::from(&common_paths::game_directory());

      // All frameworks from thunderstore just need the directory matching the name extracted
      let sub_dir = if let Ok(Manifest { name }) = self.try_parse_manifest(archive) {
        format!("{}/", name)
      } else {
        String::new()
      };

      (output_dir, sub_dir)
    } else {
      info!("Installing Mod...");
      // thunderstore mods are extracted into a subfolder in the plugin directory
      let mut output_dir = PathBuf::from(&common_paths::bepinex_plugin_directory());
      if let Ok(Manifest { name }) = self.try_parse_manifest(archive) {
        output_dir.push(name);
      }
      create_dir_all(&output_dir).unwrap_or_else(|_| {
        error!("Failed to create mod directory! {:?}", output_dir);
        // TODO: Remove Exit Code and provide an Ok or Err.
        exit(1);
      });

      (output_dir, "".to_string())
    };

    match archive.extract_sub_dir_custom(output_dir, &archive_dir) {
      Ok(_) => info!("Successfully installed {}", &self.url),
      Err(msg) => {
        error!(
          "Failed to install: {}\nDownloaded Archive: {:?}\n{}",
          self.url,
          self.staging_location,
          msg.to_string()
        );
        // TODO: Remove Exit Code and provide an Ok or Err.
        exit(1);
      }
    };
  }

  pub fn install(&mut self) {
    if Path::new(&self.staging_location).is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory! {:?}",
        self.staging_location
      );
      // TODO: Remove Exit Code and provide an Ok or Err.
      exit(1)
    }

    if self.file_type.eq("dll") {
      debug!("Copying downloaded dll to BepInEx plugin directory...");
      self.copy_single_file(
        &self.staging_location,
        &common_paths::bepinex_plugin_directory(),
      );
    } else if self.file_type.eq("cfg") {
      debug!("Copying single cfg into config directory");
      let src_file_path = &self.staging_location;
      let cfg_file_name = self.staging_location.file_name().unwrap();

      // If the cfg already exists in the output directory then append a ".new"
      let mut dst_file_path =
        Path::new(&common_paths::bepinex_config_directory()).join(cfg_file_name);
      if dst_file_path.exists() {
        dst_file_path = dst_file_path.with_extension("cfg.new");
      }

      fs::rename(src_file_path, dst_file_path).unwrap();
    } else {
      let zip_file = File::open(&self.staging_location).unwrap();
      let mut archive = match ZipArchive::new(zip_file) {
        Ok(file_archive) => {
          debug!("Successfully parsed zip file {:?}", self.staging_location);
          file_archive
        }
        Err(_) => {
          error!("Failed to parse zip file {:?}", self.staging_location);
          // TODO: Remove Exit Code and provide an Ok or Err.
          exit(1);
        }
      };
      self.extract_plugin(&mut archive);
    }
    self.installed = true
  }

  pub fn download(&mut self) -> Result<String, String> {
    debug!("Initializing mod download...");
    let download_url = &self.url.clone();
    if !Path::new(&self.staging_location).exists() {
      error!("Failed to download file to staging location!");
      return Err(format!(
        "Directory does not exist! {:?}",
        self.staging_location
      ));
    }
    if let Ok(parsed_url) = Url::parse(download_url) {
      match reqwest::blocking::get(parsed_url) {
        Ok(mut response) => {
          if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
            debug!("Using url (in case of redirect): {}", &self.url);
            self.url = response.url().to_string();
            self.file_type = url_parse_file_type(response.url().as_ref());
          }

          let file_name = parse_file_name(
            &Url::parse(&self.url).unwrap(),
            format!("{}.{}", get_md5_hash(download_url), &self.file_type).as_str(),
          );
          self.staging_location = self.staging_location.join(file_name);
          debug!("Downloading to: {:?}", self.staging_location);
          let mut file = File::create(&self.staging_location).unwrap();
          response.copy_to(&mut file).expect("Failed saving mod file");
          self.downloaded = true;
          debug!("Download Complete!: {}", &self.url);
          debug!("Download Output: {:?}", self.staging_location);
          Ok(String::from("Successful"))
        }
        Err(err) => {
          error!("Failed to download mod: {}", download_url);
          Err(err.status().unwrap().to_string())
        }
      }
    } else {
      Err(format!(
        "Failed to download mod with invalid url: {}",
        &download_url
      ))
    }
  }
}

pub mod bepinex;

use crate::utils::common_paths::{bepinex_plugin_directory, game_directory};
use crate::utils::{common_paths, create_hash, parse_file_name, path_exists};
use fs_extra::dir;
use log::{debug, error, info};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use zip::result::ZipError;
use zip::ZipArchive;

pub struct ValheimMod {
  pub(crate) url: String,
  pub(crate) staging_location: String,
  pub(crate) installed: bool,
  pub(crate) downloaded: bool,
  pub(crate) staged: bool,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
  name: String,
}

impl ValheimMod {
  pub fn new(url: &str) -> ValheimMod {
    ValheimMod {
      url: String::from(url),
      staging_location: common_paths::mods_directory(),
      installed: false,
      downloaded: false,
      staged: false,
    }
  }
  // fn uninstall(&self) {}
  fn parse_manifest(&self, archive: &mut ZipArchive<File>) -> Result<Manifest, ZipError> {
    debug!("Parsing manifest...");
    let manifest = archive.by_name("manifest.json")?;
    Ok(serde_json::from_reader(manifest).expect("Failed deserializing manifest"))
  }

  fn copy_staged_plugin(&mut self, manifest: &Manifest) {
    if !&self.staged {
      error!("Zip file not extracted to staging location!!");
      return;
    }
    let working_directory = game_directory();
    let mut staging_output = String::from(&self.staging_location);
    let sub_dir = format!("{}/{}", &staging_output, &manifest.name);
    debug!("Manifest suggests sub directory: {}", sub_dir);
    let mut dir_copy_options = dir::CopyOptions::new();
    dir_copy_options.overwrite = true;
    let mut copy_destination = bepinex_plugin_directory();
    if path_exists(&sub_dir)
      && (manifest.name.eq("BepInExPack_Valheim") || manifest.name.eq("BepInEx_Valheim_Full"))
    {
      staging_output = format!("{}/{}", &staging_output, &manifest.name);
      copy_destination = String::from(&working_directory);
    } else {
      copy_destination = format!("{}/{}", &copy_destination, &manifest.name);
      debug!("Creating mod directory: {}", &copy_destination);
      match create_dir_all(&copy_destination) {
        Ok(_) => info!("Created mod directory: {}", &copy_destination),
        Err(_) => error!("Failed to create mod directory! {}", &copy_destination),
      };
    }
    debug!(
      "Copying contents from: \n{}\nInto Directory:\n{}",
      &staging_output, &working_directory
    );
    let source_contents: Vec<_> = std::fs::read_dir(&staging_output)
      .unwrap()
      .map(|entry| entry.unwrap().path())
      .collect();
    match fs_extra::copy_items(&source_contents, &copy_destination, &dir_copy_options) {
      Ok(_) => info!("Successfully installed {}", &self.url),
      Err(_) => error!("Failed to install {}", &self.url),
    }
  }
  fn stage_plugin(&mut self, archive: &mut ZipArchive<File>) {
    let mut staging_output = String::from(
      Path::new(&self.staging_location)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap(),
    );
    staging_output = format!("{}/{}", common_paths::mods_directory(), &staging_output);
    debug!(
      "Extracting contents to staging directory: {}",
      staging_output
    );
    archive.extract(&staging_output).unwrap();
    self.staging_location = String::from(&staging_output);
    self.staged = true;
  }
  fn extract_plugin(&self, archive: &mut ZipArchive<File>) {
    let output_path = if archive
      .file_names()
      .any(|file_name| file_name.eq_ignore_ascii_case("winhttp.dll"))
    {
      info!("Installing BepInEx...");
      common_paths::game_directory()
    } else {
      info!("Installing Mod...");
      common_paths::bepinex_plugin_directory()
    };
    match archive.extract(output_path) {
      Ok(_) => info!("Successfully installed {}", &self.url),
      Err(msg) => error!(
        "Failed to install: {}\nDownloaded Archive: {}\n{}",
        &self.url,
        &self.staging_location,
        msg.to_string()
      ),
    };
  }
  pub fn install(&mut self) {
    if Path::new(&self.staging_location).is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory! {}",
        &self.staging_location
      );
      return;
    }
    let zip_file = std::fs::File::open(&self.staging_location).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    if archive
      .file_names()
      .any(|file_name| file_name.eq_ignore_ascii_case("manifest.json"))
    {
      debug!("Found manifest!! Checking instructions...");
      let manifest = self.parse_manifest(&mut archive).unwrap();
      debug!("Manifest has name: {}", manifest.name);
      self.stage_plugin(&mut archive);
      self.copy_staged_plugin(&manifest);
    } else {
      self.extract_plugin(&mut archive);
    }
    self.installed = true
  }

  pub fn download(&mut self) -> Result<String, String> {
    debug!("Initializing mod download...");
    let download_url = String::from(&self.url);
    if !Path::new(&self.staging_location).exists() {
      error!("Failed to download file to staging location!");
      return Err(format!(
        "Directory does not exist! {}",
        &self.staging_location
      ));
    }
    if let Ok(parsed_url) = Url::parse(&download_url) {
      match reqwest::blocking::get(parsed_url) {
        Ok(response) => {
          if !&self.url.ends_with(".zip") {
            debug!("Using url (in case of redirect): {}", &self.url);
            self.url = response.url().to_string();
          }
          let file_name = parse_file_name(
            &Url::parse(&self.url).unwrap(),
            format!("{}.zip", create_hash(&download_url)).as_str(),
          );
          self.staging_location = format!("{}/{}", &self.staging_location, file_name);
          debug!("Downloading to: {}", &self.staging_location);
          let file_contents = response.bytes().unwrap();
          let mut file = File::create(&self.staging_location).unwrap();
          file.write_all(&file_contents.to_vec()).unwrap();
          self.downloaded = true;
          debug!("Download Complete!: {}", &self.url);
          debug!("Download Output: {}", &self.staging_location);
          Ok(String::from("Successful"))
        }
        Err(err) => {
          error!("Failed to download mod: {}", download_url);
          Err(err.status().unwrap().to_string())
        }
      }
    } else {
      Err(String::from("Failed to download mod!"))
    }
  }
}

pub mod bepinex;
mod nexus;

use crate::constants::NEXUS_PREMIUM_API_KEY;
use crate::utils::{create_hash, parse_file_name};
use log::error;
use reqwest::Url;
use std::env;
use std::fs::File;
use std::io::Write;

pub struct ValheimMod {
  pub(crate) url: String,

  pub(crate) staging_location: String,
  pub(crate) installed: bool,
  pub(crate) downloaded: bool,
}

impl ValheimMod {
  // fn uninstall(&self) {}
  pub fn install(&self) {}

  pub fn download(&mut self) -> Result<String, String> {
    let mut download_url = String::from(&self.url);

    if env::var(NEXUS_PREMIUM_API_KEY).is_ok()
      && download_url.starts_with("https://www.nexusmods.com")
    {
      if let Ok(parsed_url) = reqwest::Url::parse(&self.url) {
        if let Ok(nexus_download_url) = nexus::fetch_download_url(&parsed_url) {
          download_url = nexus_download_url;
        };
      }
    }
    if let Ok(parsed_url) = Url::parse(&download_url) {
      match reqwest::blocking::get(parsed_url.to_owned()) {
        Ok(response) => {
          let file_contents = response.bytes().unwrap();
          let mods_dir = &self.staging_location;
          let file_name = parse_file_name(
            &parsed_url,
            format!("{}.zip", create_hash(&download_url)).as_str(),
          );
          let mut file = File::create(format!("{}/{}", mods_dir, file_name)).unwrap();
          file.write_all(&file_contents.to_vec()).unwrap();
          self.downloaded = true;
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

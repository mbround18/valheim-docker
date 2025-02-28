use crate::mods::ValheimMod;

use crate::errors::ValheimModError;
use log::{debug, error, info};
use std::process::exit;

fn process_mod(input: &str) -> Result<(), ValheimModError> {
  match ValheimMod::try_from(input.to_string()) {
    Ok(mut valheim_mod) => {
      info!("Installing {}", &input);
      debug!("Mod URL: {}", valheim_mod.url);
      match valheim_mod.download() {
        Ok(_) => {
          valheim_mod.install()?;
          Ok(())
        }
        Err(message) => {
          error!("Download failed: {}", message);
          Err(ValheimModError::DownloadFailed)
        }
      }
    }
    Err(e) => {
      error!("Invalid input: {}", e);
      Err(e)
    }
  }
}

pub fn invoke(input: String) {
  if let Err(e) = process_mod(&input) {
    error!("Failed to process mod: {}", e);
    exit(1);
  }
}

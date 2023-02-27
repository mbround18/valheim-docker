use crate::mods::ValheimMod;

use log::{debug, error, info};
use std::process::exit;

pub fn invoke(url: String) {
  let mut valheim_mod = ValheimMod::new(&url);
  info!("Installing {}", valheim_mod.url);
  debug!("Mod URL: {}", valheim_mod.url);
  debug!("Mod staging location: {:?}", valheim_mod.staging_location);
  match valheim_mod.download() {
    Ok(_) => valheim_mod.install(),
    Err(message) => {
      error!("{}", message);
      exit(1);
    }
  };
}

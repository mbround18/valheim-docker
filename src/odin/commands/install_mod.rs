use crate::mods::ValheimMod;
use clap::ArgMatches;
use log::{debug, error, info};
use std::process::exit;

pub fn invoke(args: &ArgMatches) {
  let mut valheim_mod = ValheimMod::new(args.value_of("URL").unwrap());
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

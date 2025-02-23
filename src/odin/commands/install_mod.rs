use crate::mods::ValheimMod;
use crate::utils::{is_valid_url, parse_mod_string};

use log::{debug, error, info};
use std::process::exit;

fn process_mod(url: &str, mod_name: &str, author: Option<&str>) {
  let mut valheim_mod = ValheimMod::new(url);
  if let Some(author) = author {
    info!("Installing {} by {}", mod_name, author);
  } else {
    info!("Installing {}", mod_name);
  }
  debug!("Mod URL: {}", valheim_mod.url);
  match valheim_mod.download() {
    Ok(_) => valheim_mod.install(),
    Err(message) => {
      error!("Download failed: {}", message);
      exit(1);
    }
  }
}

pub fn invoke(input: String) {
  if is_valid_url(&input) {
    process_mod(&input, &input, None);
  } else if let Some((author, mod_name, version)) = parse_mod_string(&input) {
    let constructed_url = format!(
      "https://gcdn.thunderstore.io/live/repository/packages/{}-{}-{}.zip",
      author, mod_name, version
    );
    process_mod(&constructed_url, mod_name, Some(author));
  } else {
    error!("Invalid input: {}", input);
    exit(1);
  }
}

use log::{debug, error, info};

use std::process::exit;

use crate::{constants, server, utils::get_working_dir};

pub fn invoke(dry_run: bool) {
  debug!("Stopping server, directory needs to be where the server executable is located.");
  info!(
    "Stopping server, using working directory {}",
    get_working_dir()
  );
  if dry_run {
    info!("This command would have run: ");
    info!("kill -2 {}", constants::VALHEIM_EXECUTABLE_NAME)
  } else {
    if !server::is_installed() {
      error!("Failed to find server executable!");
      exit(1);
    }
    server::blocking_shutdown();
  }
}

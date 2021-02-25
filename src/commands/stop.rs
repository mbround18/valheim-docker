use clap::ArgMatches;
use log::{error, info};

use std::process::exit;

use crate::{constants, server, utils::get_working_dir};

pub fn invoke(args: &ArgMatches) {
  info!("Stopping server {}", get_working_dir());
  if args.is_present("dry_run") {
    info!("This command would have run: ");
    info!("kill -2 {}", constants::VALHEIM_EXECUTABLE_NAME)
  } else {
    if !server::is_installed() {
      error!("Failed to find server executable!");
      exit(1);
    }
    server::send_shutdown();
    server::wait_for_exit();
  }
}

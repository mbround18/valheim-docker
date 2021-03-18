use crate::files::config::load_config;
use crate::server;
use clap::ArgMatches;
use log::{debug, error, info};
use std::process::exit;

pub fn invoke(args: &ArgMatches) {
  info!("Setting up start scripts...");
  debug!("Loading config file...");
  let config = load_config();

  let dry_run: bool = args.is_present("dry_run");
  debug!("Dry run condition: {}", dry_run);

  info!("Looking for burial mounds...");
  if !dry_run {
    match server::start_daemonized(config) {
      Ok(_) => info!("Success, daemonized"),
      Err(e) => {
        error!("Error: {}", e);
        exit(1);
      }
    }
  } else {
    info!(
      "This command would have launched\n{} -nographics -batchmode -port {} -name {} -world {} -password {} -public {}",
      &config.command,
      &config.port,
      &config.name,
      &config.world,
      &config.password,
      &config.public,
    )
  }
}

use crate::files::config::{config_file, write_config};
use clap::ArgMatches;
use log::debug;

pub fn invoke(args: &ArgMatches) {
  debug!("Pulling config file...");
  let config = config_file();
  debug!("Writing config file...");
  write_config(config, args);
}

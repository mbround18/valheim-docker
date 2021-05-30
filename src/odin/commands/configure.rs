use crate::files::config::{config_file, write_config};
use crate::files::discord::{discord_file, write_discord};
use clap::ArgMatches;
use log::debug;

pub fn invoke(args: &ArgMatches) {
  debug!("Pulling config file...");
  let config = config_file();
  debug!("Writing config file...");
  write_config(config, args);
  debug!("Pulling Discord config file...");
  let discord = discord_file();
  debug!("Writing Discord config file...");
  write_discord(discord);
}

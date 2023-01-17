use crate::files::config::{config_file, write_config};
use crate::files::discord::{discord_file, write_discord};

use log::debug;

pub struct Configuration {
  pub name: String,
  pub server_executable: String,
  pub port: u16,
  pub world: String,
  pub password: String,
  pub public: bool,
}

impl Configuration {
  pub fn new(
    name: String,
    server_executable: String,
    port: u16,
    world: String,
    password: String,
    public: bool,
  ) -> Self {
    Configuration {
      name,
      server_executable,
      port,
      world,
      password,
      public,
    }
  }
  pub fn invoke(self) {
    debug!("Pulling config file...");
    let config = config_file();
    debug!("Writing config file...");
    write_config(config, self);
    debug!("Pulling Discord config file...");
    let discord = discord_file();
    debug!("Writing Discord config file...");
    write_discord(discord);
  }
}

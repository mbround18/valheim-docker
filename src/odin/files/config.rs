use crate::commands::configure::Configuration;
use crate::files::{FileManager, ManagedFile};
use crate::traits::AsOneOrZero;
use crate::utils::environment::fetch_var;

use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::exit};

const ODIN_CONFIG_FILE_VAR: &str = "ODIN_CONFIG_FILE";

#[derive(Deserialize, Serialize)]
pub struct ValheimArguments {
  pub(crate) port: String,
  pub(crate) name: String,
  pub(crate) world: String,
  pub(crate) public: String,
  pub(crate) password: String,
  pub(crate) command: String,
}

impl From<Configuration> for ValheimArguments {
  fn from(value: Configuration) -> Self {
    let command = match fs::canonicalize(PathBuf::from(value.server_executable)) {
      Ok(command_path) => command_path.to_str().unwrap().to_string(),
      Err(_) => {
        error!("Failed to find server executable! Please run `odin install`");
        exit(1)
      }
    };

    ValheimArguments {
      port: value.port.to_string(),
      name: value.name,
      world: value.world,
      public: value.public.as_string(),
      password: value.password,
      command,
    }
  }
}

pub fn load_config() -> ValheimArguments {
  let file = config_file();
  let config = read_config(file);

  debug!("Checking password compliance...");
  if config.password.len() < 5 && !config.password.is_empty() {
    error!("The supplied password is too short! It must be 5 characters or greater!");
    exit(1);
  }
  config
}

pub fn config_file() -> ManagedFile {
  let name = fetch_var(ODIN_CONFIG_FILE_VAR, "config.json");
  debug!("Config file set to: {}", name);
  ManagedFile { name }
}

pub fn read_config(config: ManagedFile) -> ValheimArguments {
  let content = config.read();
  if content.is_empty() {
    panic!("Please initialize odin with `odin configure`. See `odin configure --help`")
  }
  serde_json::from_str(content.as_str()).unwrap()
}

pub fn write_config(config: ManagedFile, args: Configuration) -> bool {
  let content = ValheimArguments::from(args);

  let content_to_write = serde_json::to_string_pretty(&content).unwrap();
  debug!(
    "Writing config content: \n{}",
    serde_json::to_string_pretty(&content).unwrap()
  );
  config.write(content_to_write)
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::Rng;
  use std::env;
  use std::env::current_dir;

  #[test]
  #[should_panic(
    expected = "Please initialize odin with `odin configure`. See `odin configure --help`"
  )]
  fn can_read_config_panic() {
    let mut rng = rand::thread_rng();
    let n1: u8 = rng.gen();
    env::set_var(
      ODIN_CONFIG_FILE_VAR,
      format!(
        "{}/config.{}.json",
        current_dir().unwrap().to_str().unwrap(),
        n1
      ),
    );
    read_config(config_file());
  }
}

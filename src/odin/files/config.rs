use std::{fs, path::PathBuf, process::exit};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use crate::commands::configure::{Configuration, Modifiers};
use crate::files::{FileManager, ManagedFile};
use crate::traits::AsOneOrZero;
use crate::utils::environment::fetch_var;

const ODIN_CONFIG_FILE_VAR: &str = "ODIN_CONFIG_FILE";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ValheimArguments {
  /// The port of the server, (Can be set with ENV variable PORT)
  pub(crate) port: String,

  /// The name of the server, (Can be set with ENV variable NAME)
  pub(crate) name: String,

  /// The world of the server, (Can be set with ENV variable WORLD)
  pub(crate) world: String,

  /// The public state of the server, (Can be set with ENV variable PUBLIC)
  pub(crate) public: String,

  /// The password of the server, (Can be set with ENV variable PASSWORD)
  pub(crate) password: String,

  /// The command to launch the server
  pub(crate) command: String,

  /// The preset for launching the server, (Can be set with ENV variable PRESET)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) preset: Option<String>,

  /// The modifiers for launching the server, (Can be set with ENV variable MODIFIERS)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) modifiers: Option<Vec<Modifiers>>,

  /// The set_key for launching the server, (Can be set with ENV variable SET_KEY)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) set_key: Option<String>,

  /// Sets the save interval in seconds
  #[serde(skip_serializing_if = "Option::is_none")]
  pub save_interval: Option<u16>,
}

impl From<Configuration> for ValheimArguments {
  /// Creates a new ValheimArguments from a Configuration
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
      preset: value.preset,
      modifiers: value.modifiers,
      set_key: value.set_key,
      save_interval: value.save_interval,
    }
  }
}

/// Loads the configuration from the config file
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

/// Creates a new config file
pub fn config_file() -> ManagedFile {
  let name = fetch_var(ODIN_CONFIG_FILE_VAR, "config.json");
  debug!("Config file set to: {name}");
  ManagedFile { name }
}

/// Reads the config file
pub fn read_config(config: ManagedFile) -> ValheimArguments {
  let content = config.read();
  if content.is_empty() {
    panic!("Please initialize odin with `odin configure`. See `odin configure --help`")
  }
  serde_json::from_str(content.as_str()).unwrap()
}

/// Writes the config file
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
  use std::env;
  use std::env::current_dir;

  use rand::Rng;

  use super::*;

  #[test]
  #[should_panic(
    expected = "Please initialize odin with `odin configure`. See `odin configure --help`"
  )]
  fn can_read_config_panic() {
    let mut rng = rand::rng();
    let n1: u8 = rng.random();
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

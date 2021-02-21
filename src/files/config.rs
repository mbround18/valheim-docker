use crate::files::ValheimArguments;
use crate::files::{FileManager, ManagedFile};
use crate::utils::{get_variable, get_working_dir, VALHEIM_EXECUTABLE_NAME};
use clap::ArgMatches;
use log::{debug, error};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

const ODIN_CONFIG_FILE_VAR: &str = "ODIN_CONFIG_FILE";

pub fn config_file() -> ManagedFile {
  let name = env::var(ODIN_CONFIG_FILE_VAR).unwrap_or_else(|_| "config.json".to_string());
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

pub fn write_config(config: ManagedFile, args: &ArgMatches) -> bool {
  let server_executable: &str = &[get_working_dir(), VALHEIM_EXECUTABLE_NAME.to_string()].join("/");
  let command = match fs::canonicalize(PathBuf::from(get_variable(
    args,
    "server_executable",
    server_executable.to_string(),
  ))) {
    std::result::Result::Ok(command_path) => command_path.to_str().unwrap().to_string(),
    std::result::Result::Err(_) => {
      error!("Failed to find server executable! Please run `odin install`");
      exit(1)
    }
  };

  let content = &ValheimArguments {
    port: get_variable(args, "port", "2456".to_string()),
    name: get_variable(args, "name", "Valheim powered by Odin".to_string()),
    world: get_variable(args, "world", "Dedicated".to_string()),
    public: get_variable(args, "public", "1".to_string()),
    password: get_variable(args, "password", "12345".to_string()),
    command,
  };
  let content_to_write = serde_json::to_string(content).unwrap();
  debug!(
    "Writing config content: \n{}",
    serde_json::to_string(content).unwrap()
  );
  config.write(content_to_write)
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::Rng;
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

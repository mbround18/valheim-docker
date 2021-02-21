use clap::ArgMatches;
use log::debug;
use std::env;
use std::path::Path;

pub const VALHEIM_EXECUTABLE_NAME: &str = "valheim_server.x86_64";

pub fn get_working_dir() -> String {
  env::current_dir().unwrap().to_str().unwrap().to_string()
}

pub fn get_variable(args: &ArgMatches, name: &str, default: String) -> String {
  debug!("Checking env for {}", name);
  if let Ok(env_val) = env::var(name.to_uppercase()) {
    if !env_val.is_empty() {
      debug!("Env variable found {}={}", name, env_val);
      return env_val;
    }
  }
  if let Ok(env_val) = env::var(format!("SERVER_{}", name).to_uppercase()) {
    debug!("Env variable found {}={}", name, env_val);
    return env_val;
  }
  args
    .value_of(name)
    .unwrap_or_else(|| default.as_str())
    .to_string()
}

pub fn server_installed() -> bool {
  Path::new(&[get_working_dir(), VALHEIM_EXECUTABLE_NAME.to_string()].join("/")).exists()
}

pub(crate) fn fetch_env(name: &str, default: &str, is_multiple: bool) -> String {
  match env::var(name) {
    Ok(val) => {
      let formatted_value = val.replace("\"", "");
      debug!("Found env var '{}': '{}'", name, formatted_value);
      if is_multiple {
        format!(":{}", formatted_value)
      } else {
        formatted_value
      }
    }
    Err(_) => {
      debug!("Using default env var '{}': '{}'", name, default);
      default.to_string()
    }
  }
}

use clap::ArgMatches;
use log::debug;
use std::env;
use std::path::Path;

const ODIN_WORKING_DIR: &str = "ODIN_WORKING_DIR";
pub const VALHEIM_EXECUTABLE_NAME: &str = "valheim_server.x86_64";

pub fn get_working_dir() -> String {
  fetch_env(
    ODIN_WORKING_DIR,
    env::current_dir().unwrap().to_str().unwrap(),
    false,
  )
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
  let mut formatted_value = match env::var(name) {
    Ok(val) => val.replace("\"", ""),
    Err(_) => {
      debug!("Using default env var '{}': '{}'", name, default);
      default.to_string()
    }
  };
  if is_multiple && !formatted_value.is_empty() {
    formatted_value = format!("{}:", formatted_value)
  }
  debug!("Found env var '{}': '{}'", name, formatted_value);
  formatted_value
}

pub(crate) fn path_exists(path: &str) -> bool {
  let state = Path::new(path).exists();
  debug!(
    "Path {} {}",
    path,
    if state { "exists" } else { "does not exist" }
  );
  state
}

#[cfg(test)]
mod fetch_env_tests {
  use crate::utils::fetch_env;
  use std::env;

  #[test]
  fn is_multiple_false() {
    let expected_key = "is_multiple_false";
    let expected_value = "123";
    env::set_var(expected_key, expected_value);
    let observed_value = fetch_env(expected_key, "", false);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_multiple_true() {
    let expected_key = "is_multiple_true";
    let expected_value = "456";
    env::set_var(expected_key, expected_value);
    let observed_value = fetch_env(expected_key, "", false);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn has_default() {
    let expected_key = "has_default";
    let expected_value = "789";
    env::remove_var(expected_key);
    let observed_value = fetch_env(expected_key, expected_value, false);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_empty() {
    let expected_key = "is_empty";
    let expected_value = "";
    env::remove_var(expected_key);
    let observed_value = fetch_env(expected_key, expected_value, false);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_empty_multiple() {
    let expected_key = "is_empty_multiple";
    let expected_value = "";
    env::remove_var(expected_key);
    let observed_value = fetch_env(expected_key, expected_value, true);
    assert_eq!(expected_value, observed_value);
  }
}

use log::debug;
use std::env;

pub fn fetch_var(name: &str, default: &str) -> String {
  let value = env::var(name).unwrap_or_else(|_| String::from(default));
  if value.is_empty() {
    debug!("Using default env var '{}': '{}'", name, default);
    default.to_string()
  } else {
    debug!("Env var found '{}': '{}'", name, default);
    value
  }
}

pub fn fetch_multiple_var(name: &str, default: &str) -> String {
  let value = fetch_var(name, default);
  if value.is_empty() {
    value
  } else {
    format!("{}:", value)
  }
}

#[cfg(test)]
mod fetch_env_tests {
  use crate::utils::environment::{fetch_multiple_var, fetch_var};
  use std::env;

  #[test]
  fn is_multiple_false() {
    let expected_key = "is_multiple_false";
    let expected_value = "123";
    env::set_var(expected_key, expected_value);
    let observed_value = fetch_var(expected_key, "");
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_multiple_true() {
    let expected_key = "is_multiple_true";
    let expected_value = "456";
    env::set_var(expected_key, expected_value);
    let observed_value = fetch_var(expected_key, "");
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn has_default() {
    let expected_key = "has_default";
    let expected_value = "789";
    env::remove_var(expected_key);
    let observed_value = fetch_var(expected_key, expected_value);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_empty() {
    let expected_key = "is_empty";
    let expected_value = "";
    env::remove_var(expected_key);
    let observed_value = fetch_var(expected_key, expected_value);
    assert_eq!(expected_value, observed_value);
  }
  #[test]
  fn is_empty_multiple() {
    let expected_key = "is_empty_multiple";
    let expected_value = "";
    env::remove_var(expected_key);
    let observed_value = fetch_multiple_var(expected_key, expected_value);
    assert_eq!(expected_value, observed_value);
  }
}

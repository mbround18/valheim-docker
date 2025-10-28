use crate::utils::parse_truthy::parse_truthy;
use cached::proc_macro::cached;
use std::env;

pub fn fetch_var(name: &str, default: &str) -> String {
  match env::var(name) {
    Ok(value) => {
      if value.is_empty() {
        String::from(default)
      } else {
        value
      }
    }
    Err(_) => String::from(default),
  }
}

pub fn fetch_multiple_var(name: &str, default: &str) -> String {
  let value = fetch_var(name, default);
  if value.is_empty() {
    value
  } else {
    format!("{value}:")
  }
}

#[cached]
pub fn is_env_var_truthy(name: &'static str) -> bool {
  parse_truthy(&fetch_var(name, "0")).unwrap_or(false)
}

/// Variant of is_env_var_truthy that allows a boolean default when the env var is unset/empty.
/// This keeps a consistent 0/1 parsing convention while letting callers specify default.
pub fn is_env_var_truthy_with_default(name: &'static str, default: bool) -> bool {
  let def = if default { "1" } else { "0" };
  parse_truthy(&fetch_var(name, def)).unwrap_or(default)
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

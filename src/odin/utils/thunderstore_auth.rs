use crate::utils::environment::fetch_var;
use reqwest::RequestBuilder;

const THUNDERSTORE_USERNAME_VAR: &str = "THUNDERSTORE_USERNAME";
const THUNDERSTORE_PASSWORD_VAR: &str = "THUNDERSTORE_PASSWORD";
const THUNDERSTORE_HOST: &str = "thunderstore.io";

/// Returns `THUNDERSTORE_USERNAME`/`THUNDERSTORE_PASSWORD` when both are set, for
/// authenticating against Thunderstore's API (see https://thunderstore.io/api/docs/).
fn thunderstore_credentials() -> Option<(String, String)> {
  let username = fetch_var(THUNDERSTORE_USERNAME_VAR, "");
  let password = fetch_var(THUNDERSTORE_PASSWORD_VAR, "");
  if username.is_empty() || password.is_empty() {
    return None;
  }
  Some((username, password))
}

/// Attaches HTTP Basic Auth to `builder` when `url` targets thunderstore.io and
/// `THUNDERSTORE_USERNAME`/`THUNDERSTORE_PASSWORD` are both configured. Requests to
/// any other host are returned unmodified.
pub fn with_thunderstore_auth(builder: RequestBuilder, url: &str) -> RequestBuilder {
  let is_thunderstore = reqwest::Url::parse(url)
    .ok()
    .and_then(|u| {
      u.host_str()
        .map(|h| h.eq_ignore_ascii_case(THUNDERSTORE_HOST))
    })
    .unwrap_or(false);

  if !is_thunderstore {
    return builder;
  }

  match thunderstore_credentials() {
    Some((username, password)) => builder.basic_auth(username, Some(password)),
    None => builder,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  #[test]
  #[serial]
  fn no_auth_when_credentials_missing() {
    remove_var(THUNDERSTORE_USERNAME_VAR);
    remove_var(THUNDERSTORE_PASSWORD_VAR);
    assert!(thunderstore_credentials().is_none());
  }

  #[test]
  #[serial]
  fn no_auth_when_only_username_set() {
    set_var(THUNDERSTORE_USERNAME_VAR, "user");
    remove_var(THUNDERSTORE_PASSWORD_VAR);
    assert!(thunderstore_credentials().is_none());
    remove_var(THUNDERSTORE_USERNAME_VAR);
  }

  #[test]
  #[serial]
  fn auth_present_when_both_set() {
    set_var(THUNDERSTORE_USERNAME_VAR, "user");
    set_var(THUNDERSTORE_PASSWORD_VAR, "pass");
    assert_eq!(
      thunderstore_credentials(),
      Some(("user".to_string(), "pass".to_string()))
    );
    remove_var(THUNDERSTORE_USERNAME_VAR);
    remove_var(THUNDERSTORE_PASSWORD_VAR);
  }

  #[test]
  #[serial]
  fn non_thunderstore_host_untouched() {
    set_var(THUNDERSTORE_USERNAME_VAR, "user");
    set_var(THUNDERSTORE_PASSWORD_VAR, "pass");
    let client = reqwest::Client::new();
    let builder = with_thunderstore_auth(
      client.get("https://example.com/file.zip"),
      "https://example.com/file.zip",
    );
    let req = builder.build().unwrap();
    assert!(req.headers().get("authorization").is_none());
    remove_var(THUNDERSTORE_USERNAME_VAR);
    remove_var(THUNDERSTORE_PASSWORD_VAR);
  }

  #[test]
  #[serial]
  fn thunderstore_host_gets_basic_auth() {
    set_var(THUNDERSTORE_USERNAME_VAR, "user");
    set_var(THUNDERSTORE_PASSWORD_VAR, "pass");
    let client = reqwest::Client::new();
    let url = "https://thunderstore.io/package/download/Author/Mod/1.0.0/";
    let builder = with_thunderstore_auth(client.get(url), url);
    let req = builder.build().unwrap();
    assert!(req.headers().get("authorization").is_some());
    remove_var(THUNDERSTORE_USERNAME_VAR);
    remove_var(THUNDERSTORE_PASSWORD_VAR);
  }
}

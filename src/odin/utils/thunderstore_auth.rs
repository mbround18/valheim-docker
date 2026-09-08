use crate::utils::environment::fetch_var;
use reqwest::RequestBuilder;

const THUNDERSTORE_TOKEN_VAR: &str = "THUNDERSTORE_TOKEN";
const THUNDERSTORE_USERNAME_VAR: &str = "THUNDERSTORE_USERNAME";
const THUNDERSTORE_PASSWORD_VAR: &str = "THUNDERSTORE_PASSWORD";
const THUNDERSTORE_HOST: &str = "thunderstore.io";
const THUNDERSTORE_BASE_URL_VAR: &str = "THUNDERSTORE_BASE_URL";
const DEFAULT_THUNDERSTORE_BASE_URL: &str = "https://thunderstore.io";

/// Base URL for Thunderstore API and download requests. Overridable via
/// `THUNDERSTORE_BASE_URL` for mirrors and for tests that point at a local server.
/// The trailing slash is always trimmed so callers can join paths directly.
pub fn thunderstore_base_url() -> String {
  let base = fetch_var(THUNDERSTORE_BASE_URL_VAR, DEFAULT_THUNDERSTORE_BASE_URL);
  base.trim_end_matches('/').to_string()
}

/// Returns the `THUNDERSTORE_TOKEN` service account token when set. Thunderstore issues
/// these from a team's Service Accounts page (they look like `tss_...`) and expects them
/// as `Authorization: Bearer <token>`. See https://thunderstore.io/api/docs/.
fn thunderstore_token() -> Option<String> {
  let token = fetch_var(THUNDERSTORE_TOKEN_VAR, "");
  (!token.is_empty()).then_some(token)
}

/// Returns `THUNDERSTORE_USERNAME`/`THUNDERSTORE_PASSWORD` when both are set. Kept for
/// backwards compatibility with existing deployments; prefer `THUNDERSTORE_TOKEN`.
fn thunderstore_credentials() -> Option<(String, String)> {
  let username = fetch_var(THUNDERSTORE_USERNAME_VAR, "");
  let password = fetch_var(THUNDERSTORE_PASSWORD_VAR, "");
  if username.is_empty() || password.is_empty() {
    return None;
  }
  Some((username, password))
}

/// Whether `host` is thunderstore.io or one of its subdomains. Community sites
/// (`valheim.thunderstore.io`) and the package CDN (`gcdn.thunderstore.io`) are both
/// Thunderstore-operated, so credentials belong on those too.
pub fn is_thunderstore_host(host: &str) -> bool {
  host.eq_ignore_ascii_case(THUNDERSTORE_HOST)
    || host
      .len()
      .checked_sub(THUNDERSTORE_HOST.len() + 1)
      .is_some_and(|split| {
        host.as_bytes()[split] == b'.' && host[split + 1..].eq_ignore_ascii_case(THUNDERSTORE_HOST)
      })
}

/// Attaches Thunderstore credentials to `builder` when `url` targets thunderstore.io or one
/// of its subdomains. `THUNDERSTORE_TOKEN` is sent as a Bearer token and takes precedence;
/// `THUNDERSTORE_USERNAME`/`THUNDERSTORE_PASSWORD` fall back to HTTP Basic. Requests to any
/// other host are returned unmodified.
pub fn with_thunderstore_auth(builder: RequestBuilder, url: &str) -> RequestBuilder {
  let is_thunderstore = reqwest::Url::parse(url)
    .ok()
    .and_then(|u| u.host_str().map(is_thunderstore_host))
    .unwrap_or(false);

  if !is_thunderstore {
    return builder;
  }

  if let Some(token) = thunderstore_token() {
    return builder.bearer_auth(token);
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
  fn base_url_defaults_and_trims() {
    remove_var(THUNDERSTORE_BASE_URL_VAR);
    assert_eq!(thunderstore_base_url(), DEFAULT_THUNDERSTORE_BASE_URL);
    set_var(THUNDERSTORE_BASE_URL_VAR, "http://localhost:1234/");
    assert_eq!(thunderstore_base_url(), "http://localhost:1234");
    remove_var(THUNDERSTORE_BASE_URL_VAR);
  }

  fn clear_credentials() {
    remove_var(THUNDERSTORE_TOKEN_VAR);
    remove_var(THUNDERSTORE_USERNAME_VAR);
    remove_var(THUNDERSTORE_PASSWORD_VAR);
  }

  fn auth_header(url: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let req = with_thunderstore_auth(client.get(url), url)
      .build()
      .unwrap();
    req
      .headers()
      .get("authorization")
      .map(|v| v.to_str().unwrap().to_string())
  }

  #[test]
  #[serial]
  fn no_auth_when_credentials_missing() {
    clear_credentials();
    assert!(thunderstore_credentials().is_none());
    assert!(thunderstore_token().is_none());
  }

  #[test]
  #[serial]
  fn token_is_sent_as_bearer() {
    clear_credentials();
    set_var(THUNDERSTORE_TOKEN_VAR, "tss_secret");
    assert_eq!(
      auth_header("https://thunderstore.io/api/experimental/package/Author/Mod/"),
      Some("Bearer tss_secret".to_string())
    );
    clear_credentials();
  }

  #[test]
  #[serial]
  fn token_takes_precedence_over_basic() {
    clear_credentials();
    set_var(THUNDERSTORE_TOKEN_VAR, "tss_secret");
    set_var(THUNDERSTORE_USERNAME_VAR, "user");
    set_var(THUNDERSTORE_PASSWORD_VAR, "pass");
    assert_eq!(
      auth_header("https://thunderstore.io/api/experimental/package/Author/Mod/"),
      Some("Bearer tss_secret".to_string())
    );
    clear_credentials();
  }

  #[test]
  #[serial]
  fn subdomains_are_authenticated() {
    clear_credentials();
    set_var(THUNDERSTORE_TOKEN_VAR, "tss_secret");
    for url in [
      "https://valheim.thunderstore.io/api/v1/package/",
      "https://gcdn.thunderstore.io/live/repository/packages/Author-Mod-1.0.0.zip",
      "https://new.thunderstore.io/c/valheim/p/Author/Mod/",
    ] {
      assert!(auth_header(url).is_some(), "{url} should be authenticated");
    }
    clear_credentials();
  }

  #[test]
  #[serial]
  fn lookalike_hosts_are_not_authenticated() {
    clear_credentials();
    set_var(THUNDERSTORE_TOKEN_VAR, "tss_secret");
    for url in [
      "https://notthunderstore.io/api/",
      "https://thunderstore.io.evil.com/api/",
      "https://evil-thunderstore.io/api/",
    ] {
      assert!(auth_header(url).is_none(), "{url} must not receive auth");
    }
    clear_credentials();
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

  /// Live check that our Bearer header is actually accepted by Thunderstore.
  ///
  /// `/api/experimental/current-user/` is one of the few endpoints that validates
  /// credentials: it answers 200 anonymously, 200 with a real identity when a valid
  /// token is presented, and 401 when the token is bad. That last case is what proves
  /// the header name and scheme are right rather than merely being ignored.
  ///
  /// Enable with:
  ///   THUNDERSTORE_TOKEN=tss_... cargo test -p odin thunderstore_live_auth -- --ignored
  #[tokio::test]
  #[ignore]
  #[serial]
  async fn thunderstore_live_auth() {
    const CURRENT_USER: &str = "https://thunderstore.io/api/experimental/current-user/";

    let token = std::env::var(THUNDERSTORE_TOKEN_VAR).unwrap_or_default();
    if token.is_empty() {
      eprintln!("skipping live auth test; set {THUNDERSTORE_TOKEN_VAR} to enable");
      return;
    }

    let client = reqwest::Client::new();

    // A valid token authenticates as a real user.
    let body: serde_json::Value = with_thunderstore_auth(client.get(CURRENT_USER), CURRENT_USER)
      .send()
      .await
      .expect("current-user request should succeed")
      .json()
      .await
      .expect("current-user should return JSON");
    let username = body.get("username").and_then(|u| u.as_str());
    assert!(
      username.is_some_and(|u| !u.is_empty()),
      "expected an authenticated username; the token was not accepted"
    );

    // A bad token is rejected, which proves the header is being read rather than
    // silently ignored the way Basic auth is (see below).
    set_var(
      THUNDERSTORE_TOKEN_VAR,
      "tss_definitelynotarealtoken000000000000",
    );
    let status = with_thunderstore_auth(client.get(CURRENT_USER), CURRENT_USER)
      .send()
      .await
      .expect("request should complete")
      .status();
    set_var(THUNDERSTORE_TOKEN_VAR, token);
    assert_eq!(
      status,
      reqwest::StatusCode::UNAUTHORIZED,
      "an invalid Bearer token should be rejected"
    );
  }

  /// Thunderstore ignores HTTP Basic auth entirely - garbage credentials come back 200
  /// as an anonymous user rather than 401. Documents why THUNDERSTORE_TOKEN is the
  /// scheme that actually works and the username/password pair is legacy only.
  ///
  /// Enable with:
  ///   THUNDERSTORE_LIVE_TEST=1 cargo test -p odin thunderstore_live_basic_auth_ignored -- --ignored
  #[tokio::test]
  #[ignore]
  #[serial]
  async fn thunderstore_live_basic_auth_ignored() {
    if std::env::var("THUNDERSTORE_LIVE_TEST").unwrap_or_default() != "1" {
      eprintln!("skipping live basic auth test; set THUNDERSTORE_LIVE_TEST=1 to enable");
      return;
    }
    const CURRENT_USER: &str = "https://thunderstore.io/api/experimental/current-user/";

    let response = reqwest::Client::new()
      .get(CURRENT_USER)
      .basic_auth("nonsense", Some("nonsense"))
      .send()
      .await
      .expect("request should complete");

    assert_eq!(
      response.status(),
      reqwest::StatusCode::OK,
      "Basic auth is expected to be ignored, not rejected"
    );
    let body: serde_json::Value = response.json().await.expect("JSON body");
    assert!(
      body
        .get("username")
        .and_then(|u| u.as_str())
        .is_none_or(str::is_empty),
      "Basic auth should not authenticate anyone"
    );
  }
}

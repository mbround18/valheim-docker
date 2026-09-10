//! Mod repositories that `MODS` dependency strings resolve against.
//!
//! A dependency string such as `ValheimModding-Jotunn-2.30.0` does not say where the mod
//! lives. `MODS_REPOSITORY` picks the default repository for every entry, and an entry can
//! override it with a prefix: `hex:ValheimModding-Jotunn-*` or `ts:Azumatt-AzuCraftyBoxes-1.8.18`.
//!
//! Rule for adding a repository: give it a short, easy-to-type alias of two or three
//! letters (`ts`, `hex`) alongside its full name. The alias is what people put in front of
//! dozens of `MODS` lines, so it should never need more than a glance to type.
//!
//! Hexium (https://hexium.gg) serves a Thunderstore-compatible API, but it is not a drop-in
//! base URL swap: it has no `/package/download/...` route, and its package endpoint omits the
//! version list. Resolution for each repository lives in `mods::valheim_mod`; this module
//! owns naming, base URLs, host ownership and credentials.

use super::thunderstore_auth::{thunderstore_base_url, with_thunderstore_auth};
use crate::utils::environment::fetch_var;
use log::warn;
use reqwest::{RequestBuilder, Url};
use std::fmt;

pub const MODS_REPOSITORY_VAR: &str = "MODS_REPOSITORY";
const HEXIUM_TOKEN_VAR: &str = "HEXIUM_TOKEN";
const HEXIUM_BASE_URL_VAR: &str = "HEXIUM_BASE_URL";
const HEXIUM_HOST: &str = "hexium.gg";
const DEFAULT_HEXIUM_BASE_URL: &str = "https://hexium.gg";
const THUNDERSTORE_HOST: &str = "thunderstore.io";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModRepository {
  #[default]
  Thunderstore,
  Hexium,
}

impl ModRepository {
  pub const ALL: [ModRepository; 2] = [ModRepository::Thunderstore, ModRepository::Hexium];

  /// Parses a repository name as written in `MODS_REPOSITORY` or an entry prefix.
  /// Case-insensitive; accepts the short alias or the full name.
  pub fn parse(value: &str) -> Option<Self> {
    match value.trim().to_ascii_lowercase().as_str() {
      "ts" | "thunderstore" => Some(ModRepository::Thunderstore),
      "hex" | "hexium" => Some(ModRepository::Hexium),
      _ => None,
    }
  }

  /// The short prefix for `MODS` entries, e.g. `hex:Author-Mod-1.0.0`.
  pub fn alias(self) -> &'static str {
    match self {
      ModRepository::Thunderstore => "ts",
      ModRepository::Hexium => "hex",
    }
  }

  /// The default repository for unprefixed dependency strings, from `MODS_REPOSITORY`.
  /// Unset means Thunderstore; an unrecognised value warns and also falls back to it.
  pub fn from_env() -> Self {
    let raw = fetch_var(MODS_REPOSITORY_VAR, "");
    if raw.trim().is_empty() {
      return ModRepository::default();
    }
    ModRepository::parse(&raw).unwrap_or_else(|| {
      warn!(
        "Unknown {MODS_REPOSITORY_VAR}={raw:?}; expected \"thunderstore\" or \"hexium\". \
         Falling back to Thunderstore."
      );
      ModRepository::default()
    })
  }

  pub fn name(self) -> &'static str {
    match self {
      ModRepository::Thunderstore => "Thunderstore",
      ModRepository::Hexium => "Hexium",
    }
  }

  /// Base URL for API lookups, with any trailing slash trimmed.
  pub fn base_url(self) -> String {
    match self {
      ModRepository::Thunderstore => thunderstore_base_url(),
      ModRepository::Hexium => fetch_var(HEXIUM_BASE_URL_VAR, DEFAULT_HEXIUM_BASE_URL)
        .trim_end_matches('/')
        .to_string(),
    }
  }

  /// Whether `host` is operated by this repository, which is what gates credentials.
  /// A base URL override deliberately does not count: a mirror must not get our token.
  pub fn owns_host(self, host: &str) -> bool {
    match self {
      ModRepository::Thunderstore => is_host_or_subdomain(host, THUNDERSTORE_HOST),
      ModRepository::Hexium => is_host_or_subdomain(host, HEXIUM_HOST),
    }
  }

  /// Whether `url` is traffic for this repository, including its base URL override.
  /// Wider than [`ModRepository::owns_host`]: used for pacing, not credentials.
  pub fn serves(self, url: &Url) -> bool {
    if url.host_str().is_some_and(|host| self.owns_host(host)) {
      return true;
    }
    Url::parse(&self.base_url())
      .ok()
      .is_some_and(|base| base.host_str() == url.host_str() && base.port() == url.port())
  }
}

impl fmt::Display for ModRepository {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.name())
  }
}

/// Whether `host` is `domain` itself or one of its subdomains, without matching
/// lookalikes such as `evil-domain` or `domain.evil.com`.
pub(crate) fn is_host_or_subdomain(host: &str, domain: &str) -> bool {
  host.eq_ignore_ascii_case(domain)
    || host
      .len()
      .checked_sub(domain.len() + 1)
      .is_some_and(|split| {
        host.as_bytes()[split] == b'.' && host[split + 1..].eq_ignore_ascii_case(domain)
      })
}

/// Splits a `MODS` entry into its repository prefix and the rest.
///
/// `hex:Author-Mod-1.0.0` gives `(Some(Hexium), "Author-Mod-1.0.0")`. Anything without a
/// recognised prefix, URLs included, is returned unchanged with `None`.
pub fn split_repository_prefix(entry: &str) -> (Option<ModRepository>, &str) {
  if let Some((prefix, rest)) = entry.split_once(':') {
    if !rest.is_empty() && !rest.starts_with("//") {
      if let Some(repo) = ModRepository::parse(prefix) {
        return (Some(repo), rest);
      }
    }
  }
  (None, entry)
}

/// Returns `HEXIUM_TOKEN` when set. Hexium tokens look like `hexium_...` and follow the
/// same `Authorization: Bearer` scheme as Thunderstore service account tokens.
fn hexium_token() -> Option<String> {
  let token = fetch_var(HEXIUM_TOKEN_VAR, "");
  (!token.is_empty()).then_some(token)
}

/// Attaches `HEXIUM_TOKEN` as a Bearer token when `url` targets hexium.gg or one of its
/// subdomains (the `cdn.hexium.gg` download host included). Other hosts are untouched.
pub fn with_hexium_auth(builder: RequestBuilder, url: &str) -> RequestBuilder {
  let is_hexium = Url::parse(url)
    .ok()
    .and_then(|u| u.host_str().map(|h| ModRepository::Hexium.owns_host(h)))
    .unwrap_or(false);
  match hexium_token() {
    Some(token) if is_hexium => builder.bearer_auth(token),
    _ => builder,
  }
}

/// Attaches whichever repository's credentials belong to `url`'s host, if any. Each
/// repository only acts on its own hosts, so a token never reaches the other one.
pub fn with_repository_auth(builder: RequestBuilder, url: &str) -> RequestBuilder {
  with_hexium_auth(with_thunderstore_auth(builder, url), url)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  fn clear_env() {
    for var in [
      MODS_REPOSITORY_VAR,
      HEXIUM_TOKEN_VAR,
      HEXIUM_BASE_URL_VAR,
      "THUNDERSTORE_TOKEN",
      "THUNDERSTORE_USERNAME",
      "THUNDERSTORE_PASSWORD",
      "THUNDERSTORE_BASE_URL",
    ] {
      remove_var(var);
    }
  }

  fn auth_header(url: &str) -> Option<String> {
    let client = reqwest::Client::new();
    with_repository_auth(client.get(url), url)
      .build()
      .unwrap()
      .headers()
      .get("authorization")
      .map(|v| v.to_str().unwrap().to_string())
  }

  #[test]
  fn parses_names_and_aliases_case_insensitively() {
    for (input, expected) in [
      ("ts", ModRepository::Thunderstore),
      ("TS", ModRepository::Thunderstore),
      ("thunderstore", ModRepository::Thunderstore),
      ("hex", ModRepository::Hexium),
      (" Hex ", ModRepository::Hexium),
      ("HEXIUM", ModRepository::Hexium),
    ] {
      assert_eq!(ModRepository::parse(input), Some(expected), "{input}");
    }
    for input in ["", "nexus", "thunder", "hexi", "thunderstore.io"] {
      assert_eq!(ModRepository::parse(input), None, "{input}");
    }
  }

  /// Every repository's alias must be short and must round-trip through `parse`.
  #[test]
  fn aliases_are_short_and_parse_back() {
    for repo in ModRepository::ALL {
      let alias = repo.alias();
      assert!(
        (2..=3).contains(&alias.len()),
        "{repo} alias {alias:?} should be 2-3 letters"
      );
      assert_eq!(ModRepository::parse(alias), Some(repo));
      assert_eq!(ModRepository::parse(repo.name()), Some(repo));
    }
  }

  #[test]
  #[serial]
  fn from_env_defaults_to_thunderstore() {
    clear_env();
    assert_eq!(ModRepository::from_env(), ModRepository::Thunderstore);
    set_var(MODS_REPOSITORY_VAR, "");
    assert_eq!(ModRepository::from_env(), ModRepository::Thunderstore);
    clear_env();
  }

  #[test]
  #[serial]
  fn from_env_selects_hexium() {
    clear_env();
    set_var(MODS_REPOSITORY_VAR, "hexium");
    assert_eq!(ModRepository::from_env(), ModRepository::Hexium);
    set_var(MODS_REPOSITORY_VAR, "Hex");
    assert_eq!(ModRepository::from_env(), ModRepository::Hexium);
    clear_env();
  }

  #[test]
  #[serial]
  fn from_env_falls_back_on_unknown_value() {
    clear_env();
    set_var(MODS_REPOSITORY_VAR, "nexus");
    assert_eq!(ModRepository::from_env(), ModRepository::Thunderstore);
    clear_env();
  }

  #[test]
  fn splits_known_prefixes() {
    for (input, repo, rest) in [
      (
        "hex:Author-Mod-1.0.0",
        ModRepository::Hexium,
        "Author-Mod-1.0.0",
      ),
      ("hexium:Author-Mod-*", ModRepository::Hexium, "Author-Mod-*"),
      (
        "ts:Author-Mod-1.*",
        ModRepository::Thunderstore,
        "Author-Mod-1.*",
      ),
      (
        "TS:Author-Mod-1.0.0",
        ModRepository::Thunderstore,
        "Author-Mod-1.0.0",
      ),
      (
        "thunderstore:Author-Mod-1.0.0",
        ModRepository::Thunderstore,
        "Author-Mod-1.0.0",
      ),
    ] {
      assert_eq!(
        split_repository_prefix(input),
        (Some(repo), rest),
        "{input}"
      );
    }
  }

  #[test]
  fn leaves_unprefixed_entries_and_urls_alone() {
    for input in [
      "Author-Mod-1.0.0",
      "https://cdn.thunderstore.io/live/repository/packages/A-B-1.0.0.zip",
      "http://example.com/Plugin.dll",
      "hex://not-a-prefix",
      "hex:",
      "thunder:Author-Mod-1.0.0",
      "nexus:Author-Mod-1.0.0",
    ] {
      assert_eq!(split_repository_prefix(input), (None, input), "{input}");
    }
  }

  #[test]
  #[serial]
  fn hexium_base_url_defaults_and_trims() {
    clear_env();
    assert_eq!(ModRepository::Hexium.base_url(), DEFAULT_HEXIUM_BASE_URL);
    set_var(HEXIUM_BASE_URL_VAR, "http://localhost:4321/");
    assert_eq!(ModRepository::Hexium.base_url(), "http://localhost:4321");
    clear_env();
  }

  #[test]
  fn host_ownership_rejects_lookalikes() {
    let hexium = ModRepository::Hexium;
    assert!(hexium.owns_host("hexium.gg"));
    assert!(hexium.owns_host("cdn.hexium.gg"));
    assert!(hexium.owns_host("CDN.Hexium.GG"));
    for host in [
      "nothexium.gg",
      "hexium.gg.evil.com",
      "evil-hexium.gg",
      "thunderstore.io",
    ] {
      assert!(!hexium.owns_host(host), "{host}");
    }
    assert!(ModRepository::Thunderstore.owns_host("gcdn.thunderstore.io"));
    assert!(!ModRepository::Thunderstore.owns_host("hexium.gg"));
  }

  #[test]
  #[serial]
  fn serves_includes_base_url_override() {
    clear_env();
    let local = Url::parse("http://127.0.0.1:4321/api/experimental/package/A/B/").unwrap();
    assert!(!ModRepository::Hexium.serves(&local));
    set_var(HEXIUM_BASE_URL_VAR, "http://127.0.0.1:4321");
    assert!(ModRepository::Hexium.serves(&local));
    assert!(!ModRepository::Thunderstore.serves(&local));
    clear_env();
  }

  #[test]
  #[serial]
  fn hexium_token_is_sent_to_hexium_hosts_only() {
    clear_env();
    set_var(HEXIUM_TOKEN_VAR, "hexium_secret");
    for url in [
      "https://hexium.gg/api/experimental/package/A/B/1.0.0/",
      "https://cdn.hexium.gg/upload/1/1.0.0.zip",
    ] {
      assert_eq!(
        auth_header(url),
        Some("Bearer hexium_secret".into()),
        "{url}"
      );
    }
    for url in [
      "https://thunderstore.io/api/experimental/package/A/B/",
      "https://hexium.gg.evil.com/api/",
      "https://example.com/mod.zip",
    ] {
      assert_eq!(
        auth_header(url),
        None,
        "{url} must not receive the Hexium token"
      );
    }
    clear_env();
  }

  #[test]
  #[serial]
  fn tokens_never_cross_repositories() {
    clear_env();
    set_var(HEXIUM_TOKEN_VAR, "hexium_secret");
    set_var("THUNDERSTORE_TOKEN", "tss_secret");
    assert_eq!(
      auth_header("https://thunderstore.io/package/download/A/B/1.0.0/"),
      Some("Bearer tss_secret".into())
    );
    assert_eq!(
      auth_header("https://hexium.gg/api/experimental/package/A/B/1.0.0/"),
      Some("Bearer hexium_secret".into())
    );
    clear_env();
  }

  #[test]
  #[serial]
  fn hexium_token_not_sent_to_base_url_override() {
    clear_env();
    set_var(HEXIUM_TOKEN_VAR, "hexium_secret");
    set_var(HEXIUM_BASE_URL_VAR, "https://mirror.example.com");
    assert_eq!(
      auth_header("https://mirror.example.com/api/experimental/package/A/B/1.0.0/"),
      None
    );
    clear_env();
  }
}

use crate::errors::ValheimModError;
use crate::utils::common_paths;
use log::{debug, info, warn};
use reqwest::Client;
use reqwest::Url;
use std::path::PathBuf;

/// Given a ValheimPlus DLL URL, return the expected config URL by replacing
/// the filename with `valheim_plus.cfg`.
fn config_url_from_dll_url(dll_url: &str) -> Result<String, ValheimModError> {
  let mut url = Url::parse(dll_url).map_err(|_| ValheimModError::InvalidUrl)?;
  {
    let mut segs = url
      .path_segments_mut()
      .map_err(|_| ValheimModError::InvalidUrl)?;
    segs.pop_if_empty();
    segs.pop();
    segs.push("valheim_plus.cfg");
  }
  Ok(url.to_string())
}

pub fn is_valheim_plus_dll_url(url: &str) -> bool {
  // Fast path for common cases
  let lower = url.to_ascii_lowercase();
  if !lower.contains("valheimplus") {
    return false;
  }

  match Url::parse(url) {
    Ok(u) => u
      .path_segments()
      .and_then(|mut s| s.next_back())
      .map(|name| name.eq_ignore_ascii_case("ValheimPlus.dll"))
      .unwrap_or(false),
    Err(_) => lower.ends_with("valheimplus.dll"),
  }
}

/// Download the valheim_plus.cfg from the release URL implied by a ValheimPlus DLL URL
/// and place it into the BepInEx `config` directory.
pub async fn ensure_valheim_plus_config_for_dll_url(
  dll_url: &str,
) -> Result<Option<PathBuf>, ValheimModError> {
  let cfg_dir = PathBuf::from(common_paths::bepinex_config_directory());
  let dest = cfg_dir.join("valheim_plus.cfg");
  if dest.exists() {
    return Ok(None);
  }

  debug!("Attempting to derive config URL from {}", dll_url);
  let cfg_url = config_url_from_dll_url(dll_url)?;
  info!("Downloading config from: '{}'", cfg_url);

  let client = Client::new();
  let resp = client
    .get(&cfg_url)
    .send()
    .await
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
  if !resp.status().is_success() {
    warn!("Config URL returned non-success status: {}", resp.status());
    return Err(ValheimModError::DownloadError(format!(
      "status {}",
      resp.status()
    )));
  }

  let bytes = resp
    .bytes()
    .await
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

  std::fs::create_dir_all(&cfg_dir)
    .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;

  std::fs::write(&dest, &bytes).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;

  Ok(Some(dest))
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockito::Server;
  use serial_test::serial;
  use tempfile::tempdir;

  #[test]
  fn test_config_url_from_dll_url_valid() {
    let dll_url =
      "https://github.com/nexus-mods/valheimplus/releases/download/v1.0.0/ValheimPlus.dll";
    let result = config_url_from_dll_url(dll_url);
    assert!(result.is_ok());
    let cfg_url = result.unwrap();
    assert!(cfg_url.contains("valheim_plus.cfg"));
    assert!(!cfg_url.contains("ValheimPlus.dll"));
  }

  #[test]
  fn test_config_url_from_dll_url_invalid() {
    let invalid_url = "not a valid url at all";
    let result = config_url_from_dll_url(invalid_url);
    assert!(result.is_err());
  }

  #[test]
  fn test_is_valheim_plus_dll_url_true_cases() {
    assert!(is_valheim_plus_dll_url(
      "https://example.com/ValheimPlus.dll"
    ));
    assert!(is_valheim_plus_dll_url(
      "https://example.com/VALHEIMPLUS.DLL"
    ));
    assert!(is_valheim_plus_dll_url("http://path/valheimplus.dll"));
    assert!(is_valheim_plus_dll_url("file:///local/ValheimPlus.dll"));
  }

  #[test]
  fn test_is_valheim_plus_dll_url_false_cases() {
    assert!(!is_valheim_plus_dll_url("https://example.com/SomeMod.dll"));
    assert!(!is_valheim_plus_dll_url(
      "https://example.com/valheim_mod.dll"
    ));
    assert!(!is_valheim_plus_dll_url(
      "https://notvalheimplus.com/mod.dll"
    ));
    assert!(!is_valheim_plus_dll_url("https://valheimplus.com/file.txt"));
  }

  #[test]
  fn test_is_valheim_plus_dll_url_fast_path() {
    assert!(!is_valheim_plus_dll_url(
      "https://example.com/randommod.dll"
    ));
  }

  #[test]
  fn test_is_valheim_plus_dll_url_missing_file_name() {
    assert!(!is_valheim_plus_dll_url(
      "https://valheimplus.com/some/path/"
    ));
  }

  #[test]
  fn test_is_valheim_plus_dll_url_mixed_case() {
    assert!(is_valheim_plus_dll_url(
      "https://example.com/ValheimPLUS.dll"
    ));
    assert!(is_valheim_plus_dll_url(
      "https://example.com/VALHEIMPLUS.DLL"
    ));
    assert!(is_valheim_plus_dll_url(
      "https://example.com/valheimplus.DLL"
    ));
  }

  #[tokio::test]
  #[serial]
  async fn test_ensure_valheim_plus_config_returns_none_if_exists() {
    let tmp = tempdir().unwrap();
    let game_dir = tmp.path();
    std::env::set_var(crate::constants::GAME_LOCATION, game_dir);

    let cfg_dir = PathBuf::from(common_paths::bepinex_config_directory());
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let dest = cfg_dir.join("valheim_plus.cfg");
    std::fs::write(&dest, "existing config").unwrap();

    let result =
      ensure_valheim_plus_config_for_dll_url("https://example.com/ValheimPlus.dll").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }

  #[tokio::test]
  #[serial]
  async fn test_ensure_valheim_plus_config_downloads() {
    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("dummy")
      .create();

    let _m2 = server
      .mock("GET", "/valheim_plus.cfg")
      .with_status(200)
      .with_header("content-type", "text/plain")
      .with_body("test config content")
      .create();

    let tmp = tempdir().unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, tmp.path());

    let dll_url = format!("{}/ValheimPlus.dll", server.url());
    let result = ensure_valheim_plus_config_for_dll_url(&dll_url).await;

    assert!(result.is_ok());
    let dest = result.unwrap().expect("should have downloaded");
    assert!(dest.exists());
    let contents = std::fs::read_to_string(&dest).unwrap();
    assert_eq!(contents, "test config content");
  }

  #[tokio::test]
  #[serial]
  async fn test_ensure_valheim_plus_config_invalid_url() {
    let result = ensure_valheim_plus_config_for_dll_url("not a url").await;
    assert!(result.is_err());
  }

  #[tokio::test]
  #[serial]
  async fn test_ensure_valheim_plus_config_download_failed() {
    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .create();

    let _m2 = server
      .mock("GET", "/valheim_plus.cfg")
      .with_status(404)
      .create();

    let tmp = tempdir().unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, tmp.path());

    let dll_url = format!("{}/ValheimPlus.dll", server.url());
    let result = ensure_valheim_plus_config_for_dll_url(&dll_url).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  #[serial]
  async fn test_config_url_from_dll_various_paths() {
    let test_cases = vec![
      (
        "https://api.github.com/repos/owner/repo/releases/download/v1.0/ValheimPlus.dll",
        "valheim_plus.cfg",
      ),
      (
        "https://nexusmods.com/files/123456/ValheimPlus.dll",
        "valheim_plus.cfg",
      ),
    ];

    for (dll_url, expected_cfg) in test_cases {
      let result = config_url_from_dll_url(dll_url);
      assert!(result.is_ok(), "Failed for URL: {}", dll_url);
      let cfg_url = result.unwrap();
      assert!(
        cfg_url.contains(expected_cfg),
        "URL {} should contain {}",
        cfg_url,
        expected_cfg
      );
    }
  }

  #[tokio::test]
  #[serial]
  async fn test_ensure_valheim_plus_config_creates_directory() {
    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("dummy")
      .create();

    let _m2 = server
      .mock("GET", "/valheim_plus.cfg")
      .with_status(200)
      .with_header("content-type", "text/plain")
      .with_body("config content")
      .create();

    let tmp = tempdir().unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, tmp.path());

    let dll_url = format!("{}/ValheimPlus.dll", server.url());
    let result = ensure_valheim_plus_config_for_dll_url(&dll_url).await;

    assert!(result.is_ok());
    let dest = result.unwrap();
    assert!(dest.is_some());
    assert!(dest.unwrap().parent().unwrap().exists());
  }

  #[test]
  fn test_config_url_preserves_authority() {
    let test_urls = vec![
      "https://api.github.com/repo/file/ValheimPlus.dll",
      "https://example.com:8080/path/ValheimPlus.dll",
      "http://localhost/ValheimPlus.dll",
    ];

    for url in test_urls {
      let result = config_url_from_dll_url(url);
      assert!(result.is_ok(), "Should parse: {}", url);
      let cfg_url = result.unwrap();
      assert!(cfg_url.starts_with("https://") || cfg_url.starts_with("http://"));
    }
  }
}

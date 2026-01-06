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

  #[tokio::test]
  #[serial]
  async fn synthetic_download_valheim_plus_cfg() {
    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DUMMYDLL")
      .create();

    let _m2 = server
      .mock("GET", "/valheim_plus.cfg")
      .with_status(200)
      .with_header("content-type", "text/plain")
      .with_body("dummy cfg content")
      .create();

    // Isolate bepinex config path
    let tmp = tempdir().unwrap();
    let game = tmp.path();
    std::env::set_var(crate::constants::GAME_LOCATION, game);

    let dll_url = format!("{}/ValheimPlus.dll", server.url());

    let r = ensure_valheim_plus_config_for_dll_url(&dll_url).await;
    assert!(r.is_ok(), "{:?}", r.err());
    let dest = r.unwrap().expect("should download");
    let contents = std::fs::read_to_string(dest).unwrap();
    assert_eq!(contents, "dummy cfg content");
  }

  #[tokio::test]
  #[serial]
  async fn skips_when_config_exists() {
    let tmp = tempdir().unwrap();
    let game = tmp.path();
    std::env::set_var(crate::constants::GAME_LOCATION, game);

    let cfg_dir = PathBuf::from(common_paths::bepinex_config_directory());
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let dest = cfg_dir.join("valheim_plus.cfg");
    std::fs::write(&dest, "already").unwrap();

    let r = ensure_valheim_plus_config_for_dll_url("https://example.com/ValheimPlus.dll").await;
    assert!(r.is_ok());
    assert!(r.unwrap().is_none());
  }
}

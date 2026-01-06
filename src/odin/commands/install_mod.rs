use crate::mods::ValheimMod;
use crate::mods::{ensure_valheim_plus_config_for_dll_url, is_valheim_plus_dll_url};

use crate::errors::ValheimModError;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;

async fn process_mod(input: &str) -> Result<(), ValheimModError> {
  let mut valheim_mod = ValheimMod::async_from_url(input).await?;
  info!("Installing {}", &input);
  debug!("Mod URL: {}", valheim_mod.url);
  match valheim_mod.download().await {
    Ok(_) => {
      valheim_mod.install()?;
      Ok(())
    }
    Err(message) => {
      error!("Download failed: {message}");
      Err(ValheimModError::DownloadFailed)
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FromVarState {
  schema_version: u32,
  mods: Vec<InstalledModState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InstalledModState {
  url: String,
  file_type: String,
  staging_path: String,
  sha256: Option<String>,
  installed_paths: Vec<String>,
}

fn from_var_state_path() -> PathBuf {
  PathBuf::from(crate::utils::common_paths::mods_staging_directory()).join("from-var-mods.json")
}

fn sha256_hex(path: &Path) -> Result<String, ValheimModError> {
  let mut file =
    std::fs::File::open(path).map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 8192];
  loop {
    let n = std::io::Read::read(&mut file, &mut buf)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

fn sha_sidecar_path(staging_path: &Path) -> PathBuf {
  // Mirrors the convention used by ValheimMod::download() -> write_sha_sidecar
  let mut p = staging_path.to_path_buf();
  let file_name = staging_path
    .file_name()
    .and_then(|s| s.to_str())
    .unwrap_or("artifact");
  p.set_file_name(format!("{}.sha256", file_name));
  p
}

fn load_from_var_state() -> Result<Option<FromVarState>, ValheimModError> {
  let path = from_var_state_path();
  if !path.exists() {
    return Ok(None);
  }
  let content =
    std::fs::read_to_string(&path).map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
  let parsed: FromVarState = serde_json::from_str(&content)
    .map_err(|e| ValheimModError::ManifestDeserializeError(e.to_string()))?;
  Ok(Some(parsed))
}

fn save_from_var_state(state: &FromVarState) -> Result<(), ValheimModError> {
  let path = from_var_state_path();
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
  }
  let tmp = path.with_extension("json.tmp");
  let serialized = serde_json::to_string_pretty(state)
    .map_err(|e| ValheimModError::ManifestDeserializeError(e.to_string()))?;
  std::fs::write(&tmp, serialized).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
  std::fs::rename(&tmp, &path).map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
  Ok(())
}

fn is_safe_cleanup_path(path: &Path) -> bool {
  let plugin_root = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory());
  let game_root = PathBuf::from(crate::utils::common_paths::game_directory());

  // Only allow removing content within the game dir, and prefer plugin-root deletions.
  // This prevents accidental deletion if metadata is corrupted.
  path.starts_with(&plugin_root) || path.starts_with(&game_root)
}

fn cleanup_removed_mod(mod_state: &InstalledModState) {
  info!("Cleaning up removed mod: {}", mod_state.url);

  for p in &mod_state.installed_paths {
    let path = PathBuf::from(p);
    if !is_safe_cleanup_path(&path) {
      warn!(
        "Refusing to remove path outside allowed roots: {:?} (from {})",
        path, mod_state.url
      );
      continue;
    }
    if !path.exists() {
      continue;
    }
    let res = if path.is_dir() {
      std::fs::remove_dir_all(&path)
    } else {
      std::fs::remove_file(&path)
    };
    if let Err(e) = res {
      warn!("Failed to remove {:?}: {}", path, e);
    }
  }

  let staging = PathBuf::from(&mod_state.staging_path);
  if staging.exists() {
    if let Some(expected) = &mod_state.sha256 {
      if let Ok(actual) = sha256_hex(&staging) {
        if &actual != expected {
          warn!(
            "Staging artifact hash mismatch for {}: expected {}, got {}",
            mod_state.url, expected, actual
          );
        }
      }
    }
    if let Err(e) = std::fs::remove_file(&staging) {
      warn!("Failed to remove staging artifact {:?}: {}", staging, e);
    }
  }

  let sidecar = sha_sidecar_path(&staging);
  if sidecar.exists() {
    let _ = std::fs::remove_file(sidecar);
  }
}

async fn process_mod_collect_state(input: &str) -> Result<InstalledModState, ValheimModError> {
  let mut valheim_mod = ValheimMod::async_from_url(input).await?;
  info!("Installing {}", &input);
  debug!("Mod URL: {}", valheim_mod.url);

  valheim_mod.download().await.map_err(|message| {
    error!("Download failed: {message}");
    ValheimModError::DownloadFailed
  })?;

  let installed_paths = valheim_mod.install_with_report()?;
  let staging = valheim_mod.staging_location.clone();
  let sha = sha256_hex(&staging).ok();

  Ok(InstalledModState {
    url: input.to_string(),
    file_type: valheim_mod.file_type.clone(),
    staging_path: staging.to_string_lossy().into(),
    sha256: sha,
    installed_paths: installed_paths
      .into_iter()
      .map(|p| p.to_string_lossy().into())
      .collect(),
  })
}

pub async fn invoke(url: Option<String>, from_var: bool) {
  // We're already in a tokio runtime (from #[tokio::main]), so just run the async functions directly
  let result = if from_var {
    process_mods_from_env().await
  } else {
    match url {
      Some(u) => process_mod(&u).await,
      None => Err(ValheimModError::InvalidUrl),
    }
  };

  if let Err(e) = result {
    error!("Failed to process mod(s): {e}");
    exit(1);
  }
}

async fn process_mods_from_env() -> Result<(), ValheimModError> {
  let mods_raw = env::var("MODS").unwrap_or_default();

  // MODS supports comma and newline separated lists; treat any whitespace as a separator too.
  let normalized = mods_raw.replace([',', '\n', '\r', '\t'], " ");
  let desired_mods: Vec<String> = normalized
    .split_whitespace()
    .filter(|s| !s.trim().is_empty())
    .map(|s| s.to_string())
    .collect();

  // Load previous state so we can reconcile removed mods.
  let previous_state = load_from_var_state().unwrap_or_else(|e| {
    warn!("Failed reading from-var state; continuing without cleanup: {e}");
    None
  });

  if let Some(prev) = &previous_state {
    let desired_set: std::collections::HashSet<&str> =
      desired_mods.iter().map(|s| s.as_str()).collect();
    for old in &prev.mods {
      if !desired_set.contains(old.url.as_str()) {
        cleanup_removed_mod(old);
      }
    }
  }

  if desired_mods.is_empty() {
    info!("No MODS entries after parsing; completed cleanup reconciliation.");
    let empty = FromVarState {
      schema_version: 1,
      mods: vec![],
    };
    // Persist empty state so subsequent runs don't keep trying to clean up.
    let _ = save_from_var_state(&empty);
    return Ok(());
  }

  info!("Installing {} mod(s) from MODS env", desired_mods.len());

  let mut valheim_plus_dll_url: Option<String> = None;
  let mut new_states: Vec<InstalledModState> = Vec::with_capacity(desired_mods.len());

  // Download and install mods (async but sequential to avoid runtime issues with tests)
  // For true parallelism with 50+ mods, this could use tokio::task::JoinSet
  // but that requires multi-threaded runtime which complicates testing
  for m in &desired_mods {
    if is_valheim_plus_dll_url(m) {
      valheim_plus_dll_url = Some(m.to_string());
    }
    let state = process_mod_collect_state(m).await?;
    new_states.push(state);
  }

  // After all mods are installed, ensure ValheimPlus config exists when ValheimPlus.dll was installed.
  if let Some(dll_url) = valheim_plus_dll_url {
    match ensure_valheim_plus_config_for_dll_url(&dll_url).await {
      Ok(Some(path)) => info!("ValheimPlus config downloaded to: {:?}", path),
      Ok(None) => info!("ValheimPlus config already present; skipping download"),
      Err(e) => error!("ValheimPlus config download failed: {e}"),
    }
  }

  let state = FromVarState {
    schema_version: 1,
    mods: new_states,
  };
  save_from_var_state(&state)?;
  Ok(())
}

#[cfg(test)]
mod from_var_state_tests {
  use super::*;
  use mockito::Server;
  use serial_test::serial;
  use std::io::{Cursor, Write};

  fn make_test_zip() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    {
      let cursor = Cursor::new(&mut buf);
      let mut zipw = zip::ZipWriter::new(cursor);
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      zipw.start_file("manifest.json", options).unwrap();
      zipw.write_all(b"{\"name\":\"testmod\"}").unwrap();
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      zipw.start_file("plugins/myplugin.dll", options).unwrap();
      zipw.write_all(b"plugindata").unwrap();
      zipw.finish().unwrap();
    }
    buf
  }

  #[tokio::test]
  #[serial]
  async fn from_var_reconciles_removed_mods_and_staging() {
    let mut server = Server::new_async().await;
    let zip_bytes = make_test_zip();

    let _zip_mock = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_bytes)
      .create();

    let _dll_mock = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DUMMYDLL")
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let zip_url = format!("{}/testmod.zip", server.url());
    let dll_url = format!("{}/ValheimPlus.dll", server.url());

    // First run installs both mods.
    env::set_var("MODS", format!("{} {}", zip_url, dll_url));
    process_mods_from_env().await.expect("first run");

    let testmod_plugin = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory())
      .join("testmod")
      .join("myplugin.dll");
    assert!(testmod_plugin.exists(), "zip-installed plugin should exist");

    let vplus_plugin =
      PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory()).join("ValheimPlus.dll");
    assert!(vplus_plugin.exists(), "dll-installed plugin should exist");

    let prev_state = load_from_var_state()
      .unwrap()
      .expect("state file should exist");
    assert_eq!(prev_state.schema_version, 1);
    assert_eq!(prev_state.mods.len(), 2);

    let removed_zip = prev_state
      .mods
      .iter()
      .find(|m| m.url == zip_url)
      .expect("zip state");
    let removed_zip_staging = PathBuf::from(&removed_zip.staging_path);
    assert!(
      removed_zip_staging.exists(),
      "zip staging should exist after install"
    );
    let removed_zip_sidecar = sha_sidecar_path(&removed_zip_staging);
    assert!(
      removed_zip_sidecar.exists(),
      "zip sha sidecar should exist after download"
    );

    // Second run removes the zip mod from MODS, leaving only the DLL.
    env::set_var("MODS", &dll_url);
    process_mods_from_env().await.expect("second run");

    // Zip-installed plugin directory should be removed.
    let testmod_dir =
      PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory()).join("testmod");
    assert!(
      !testmod_dir.exists(),
      "zip-installed mod dir should be removed"
    );

    // Zip staging artifact and sidecar should be removed.
    assert!(
      !removed_zip_staging.exists(),
      "removed zip staging artifact should be deleted"
    );
    assert!(
      !removed_zip_sidecar.exists(),
      "removed zip sha sidecar should be deleted"
    );

    // Remaining DLL should still exist.
    assert!(vplus_plugin.exists(), "remaining dll plugin should exist");

    let new_state = load_from_var_state()
      .unwrap()
      .expect("state file should exist");
    assert_eq!(new_state.mods.len(), 1);
    assert_eq!(new_state.mods[0].url, dll_url);
  }

  #[tokio::test]
  #[serial]
  async fn from_var_removes_dll_when_removed_from_mods() {
    let mut server = Server::new_async().await;
    let zip_bytes = make_test_zip();

    let _zip_mock = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_bytes)
      .create();

    let _dll_mock = server
      .mock("GET", "/CustomPlugin.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("CUSTOMDLL")
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let zip_url = format!("{}/testmod.zip", server.url());
    let dll_url = format!("{}/CustomPlugin.dll", server.url());

    // Install both
    env::set_var("MODS", format!("{} {}", zip_url, dll_url));
    process_mods_from_env().await.expect("install both");

    let dll_plugin = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory())
      .join("CustomPlugin.dll");
    assert!(dll_plugin.exists(), "dll plugin should exist after install");

    let state = load_from_var_state().unwrap().expect("state file");
    let dll_state = state
      .mods
      .iter()
      .find(|m| m.url == dll_url)
      .expect("dll state");
    let dll_staging = PathBuf::from(&dll_state.staging_path);
    let dll_sidecar = sha_sidecar_path(&dll_staging);

    assert!(dll_staging.exists(), "dll staging should exist");
    assert!(dll_sidecar.exists(), "dll sidecar should exist");

    // Remove DLL from MODS
    env::set_var("MODS", &zip_url);
    process_mods_from_env().await.expect("remove dll");

    // DLL plugin, staging, and sidecar should all be gone
    assert!(
      !dll_plugin.exists(),
      "dll plugin should be removed when removed from MODS"
    );
    assert!(
      !dll_staging.exists(),
      "dll staging artifact should be removed"
    );
    assert!(!dll_sidecar.exists(), "dll sidecar should be removed");

    // ZIP mod should remain
    let testmod_plugin = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory())
      .join("testmod")
      .join("myplugin.dll");
    assert!(testmod_plugin.exists(), "zip plugin should remain");

    let final_state = load_from_var_state().unwrap().expect("state file");
    assert_eq!(final_state.mods.len(), 1);
    assert_eq!(final_state.mods[0].url, zip_url);
  }

  #[tokio::test]
  #[serial]
  async fn from_var_removes_all_mods_when_mods_becomes_empty() {
    let mut server = Server::new_async().await;
    let zip_bytes = make_test_zip();

    let _zip_mock = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_bytes)
      .create();

    let _dll_mock = server
      .mock("GET", "/AnotherMod.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("ANOTHERDLL")
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let zip_url = format!("{}/testmod.zip", server.url());
    let dll_url = format!("{}/AnotherMod.dll", server.url());

    // Install both
    env::set_var("MODS", format!("{} {}", zip_url, dll_url));
    process_mods_from_env().await.expect("install both");

    let testmod_dir =
      PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory()).join("testmod");
    let dll_plugin =
      PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory()).join("AnotherMod.dll");
    assert!(testmod_dir.exists(), "zip mod should exist");
    assert!(dll_plugin.exists(), "dll mod should exist");

    let state = load_from_var_state().unwrap().expect("state file");
    assert_eq!(state.mods.len(), 2);

    // Clear MODS
    env::set_var("MODS", "");
    process_mods_from_env().await.expect("clear mods");

    // Both should be removed
    assert!(
      !testmod_dir.exists(),
      "zip mod should be removed when MODS is empty"
    );
    assert!(
      !dll_plugin.exists(),
      "dll mod should be removed when MODS is empty"
    );

    let final_state = load_from_var_state()
      .unwrap()
      .expect("state file should still exist");
    assert_eq!(final_state.mods.len(), 0, "state should record zero mods");
  }

  #[tokio::test]
  #[serial]
  async fn from_var_handles_reinstall_after_removal() {
    let mut server = Server::new_async().await;
    let zip_bytes = make_test_zip();

    let _zip_mock = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_bytes)
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let zip_url = format!("{}/testmod.zip", server.url());

    // Install
    env::set_var("MODS", &zip_url);
    process_mods_from_env().await.expect("first install");

    let testmod_plugin = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory())
      .join("testmod")
      .join("myplugin.dll");
    assert!(
      testmod_plugin.exists(),
      "zip plugin should exist after first install"
    );

    let state_v1 = load_from_var_state().unwrap().expect("state file");
    let staging_v1 = PathBuf::from(&state_v1.mods[0].staging_path);

    // Remove mod
    env::set_var("MODS", "");
    process_mods_from_env().await.expect("remove");

    assert!(
      !testmod_plugin.exists(),
      "zip plugin should be removed after clearing MODS"
    );
    assert!(
      !staging_v1.exists(),
      "staging should be removed after clearing MODS"
    );

    // Reinstall
    env::set_var("MODS", &zip_url);
    process_mods_from_env().await.expect("reinstall");

    assert!(
      testmod_plugin.exists(),
      "zip plugin should exist after reinstall"
    );

    let state_v2 = load_from_var_state()
      .unwrap()
      .expect("state file after reinstall");
    assert_eq!(state_v2.mods.len(), 1);
    assert_eq!(state_v2.mods[0].url, zip_url);
  }

  #[tokio::test]
  #[serial]
  async fn from_var_preserves_user_modifications_to_installed_mods() {
    // This test confirms that if a user modifies files in an installed mod directory,
    // we don't interfere with those modifications. We only clean up if they remove the mod from MODS.
    let mut server = Server::new_async().await;
    let zip_bytes = make_test_zip();

    let _zip_mock = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_bytes)
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let zip_url = format!("{}/testmod.zip", server.url());

    // Install
    env::set_var("MODS", &zip_url);
    process_mods_from_env().await.expect("install");

    let testmod_dir =
      PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory()).join("testmod");
    let user_config = testmod_dir.join("user_config.txt");

    // User adds a custom config file
    std::fs::write(&user_config, "user customization data").unwrap();
    assert!(user_config.exists(), "user config should exist");

    // Run again with same MODS (no changes)
    env::set_var("MODS", &zip_url);
    process_mods_from_env().await.expect("reinstall same mod");

    // User's custom file should still be there (we don't touch it if mod stays in MODS)
    assert!(
      user_config.exists(),
      "user config should remain after reinstall of same mod"
    );

    // Now remove the mod from MODS
    env::set_var("MODS", "");
    process_mods_from_env().await.expect("remove");

    // Entire directory including user customizations should be gone
    assert!(
      !testmod_dir.exists(),
      "entire mod directory (including user mods) should be removed when mod is removed from MODS"
    );
  }

  #[tokio::test]
  #[serial]
  async fn from_var_handles_multiple_dlls() {
    let mut server = Server::new_async().await;

    let _dll1_mock = server
      .mock("GET", "/Plugin1.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DLL1DATA")
      .create();

    let _dll2_mock = server
      .mock("GET", "/Plugin2.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DLL2DATA")
      .create();

    let _dll3_mock = server
      .mock("GET", "/Plugin3.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DLL3DATA")
      .create();

    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let dll1_url = format!("{}/Plugin1.dll", server.url());
    let dll2_url = format!("{}/Plugin2.dll", server.url());
    let dll3_url = format!("{}/Plugin3.dll", server.url());

    // Install all three
    env::set_var("MODS", format!("{} {} {}", dll1_url, dll2_url, dll3_url));
    process_mods_from_env().await.expect("install three dlls");

    let plugin_dir = PathBuf::from(crate::utils::common_paths::bepinex_plugin_directory());
    assert!(plugin_dir.join("Plugin1.dll").exists());
    assert!(plugin_dir.join("Plugin2.dll").exists());
    assert!(plugin_dir.join("Plugin3.dll").exists());

    let state = load_from_var_state().unwrap().expect("state file");
    assert_eq!(state.mods.len(), 3);

    // Remove the middle one
    env::set_var("MODS", format!("{} {}", dll1_url, dll3_url));
    process_mods_from_env().await.expect("remove plugin2");

    assert!(
      plugin_dir.join("Plugin1.dll").exists(),
      "plugin1 should remain"
    );
    assert!(
      !plugin_dir.join("Plugin2.dll").exists(),
      "plugin2 should be removed"
    );
    assert!(
      plugin_dir.join("Plugin3.dll").exists(),
      "plugin3 should remain"
    );

    let final_state = load_from_var_state().unwrap().expect("state file");
    assert_eq!(final_state.mods.len(), 2);
    assert!(final_state.mods.iter().any(|m| m.url == dll1_url));
    assert!(final_state.mods.iter().any(|m| m.url == dll3_url));
    assert!(!final_state.mods.iter().any(|m| m.url == dll2_url));
  }
}

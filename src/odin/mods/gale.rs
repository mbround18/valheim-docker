//! Gale profile sync (https://github.com/Kesomannen/gale/wiki/Profile-sync).
//!
//! A Gale user can publish a mod profile and hand out its sync code. `GALE_SYNC_CODE` makes the
//! server install exactly the mods (and versions) of that profile, so clients who pull the same
//! code never hit a version mismatch. The profile is read without authentication:
//!
//! - `GET {GALE_SYNC_URL}/profile/{code}/meta` returns JSON whose `manifest` lists every mod.
//! - `GET {GALE_SYNC_URL}/profile/{code}` returns the profile zip, which carries `BepInEx/config`.
//!
//! Enabled mods become `ts:`/`hex:` entries that are merged ahead of `MODS`, so `MODS` can still
//! add server-only mods or pin a different version of a profile mod. `GALE_SYNC_CONFIGS=true`
//! also copies the profile's `BepInEx/config` files over the server's.

use crate::errors::ValheimModError;
use crate::utils::common_paths::{bepinex_config_directory, mods_staging_directory};
use crate::utils::environment::{fetch_var, is_env_var_truthy_with_default};
use crate::utils::{parse_mod_string, send_with_backoff, split_repository_prefix, ModRepository};
use log::{debug, info, warn};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

pub const GALE_SYNC_CODE_VAR: &str = "GALE_SYNC_CODE";
pub const GALE_SYNC_CONFIGS_VAR: &str = "GALE_SYNC_CONFIGS";
const GALE_SYNC_URL_VAR: &str = "GALE_SYNC_URL";
const DEFAULT_GALE_SYNC_URL: &str = "https://gale.kesomannen.com/api";

/// The container installs BepInEx itself (`TYPE=BepInEx`), so the pack entry is skipped.
const BEPINEX_PACK_NAME: &str = "BepInExPack_Valheim";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GaleVersion {
  pub major: u64,
  pub minor: u64,
  pub patch: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GaleMod {
  /// `Author-Name`, without the version.
  pub name: String,
  #[serde(alias = "versionNumber")]
  pub version: GaleVersion,
  #[serde(default = "enabled_by_default")]
  pub enabled: bool,
  /// `Thunderstore` or `Hexium`; Gale omits it for Thunderstore in older profiles.
  #[serde(default)]
  pub source: Option<String>,
}

fn enabled_by_default() -> bool {
  true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GaleManifest {
  #[serde(default)]
  pub profile_name: Option<String>,
  pub mods: Vec<GaleMod>,
}

#[derive(Debug, Deserialize)]
struct GaleProfileMeta {
  manifest: GaleManifest,
}

/// The trimmed `GALE_SYNC_CODE`, if one is set.
pub fn gale_sync_code() -> Option<String> {
  let code = fetch_var(GALE_SYNC_CODE_VAR, "");
  let code = code.trim();
  (!code.is_empty()).then(|| code.to_string())
}

fn gale_base_url() -> String {
  fetch_var(GALE_SYNC_URL_VAR, DEFAULT_GALE_SYNC_URL)
    .trim_end_matches('/')
    .to_string()
}

/// `{base}/profile/{code}[/meta]`, with the code percent-encoded as a single path segment
/// (base64 codes may contain `/` or `+`).
fn profile_url(code: &str, meta: bool) -> Result<Url, ValheimModError> {
  let mut url = Url::parse(&gale_base_url()).map_err(|_| ValheimModError::InvalidUrl)?;
  {
    let mut segments = url
      .path_segments_mut()
      .map_err(|_| ValheimModError::InvalidUrl)?;
    segments.pop_if_empty().push("profile").push(code);
    if meta {
      segments.push("meta");
    }
  }
  Ok(url)
}

/// Where the last manifest fetched for a code is kept, so a Gale outage does not stop the
/// server from booting with the mods it already had.
fn cached_manifest_path(code: &str) -> PathBuf {
  let safe: String = code
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
    .collect();
  PathBuf::from(mods_staging_directory()).join(format!("gale-{safe}.json"))
}

async fn get(url: &Url) -> Result<reqwest::Response, ValheimModError> {
  let response = send_with_backoff("gale", |client| client.get(url.clone()))
    .await
    .map_err(ValheimModError::DownloadError)?;
  let status = response.status();
  if status == reqwest::StatusCode::NOT_FOUND {
    return Err(ValheimModError::DownloadError(format!(
      "Gale profile not found at {url}; check {GALE_SYNC_CODE_VAR}"
    )));
  }
  if !status.is_success() {
    return Err(ValheimModError::DownloadError(format!(
      "Gale returned {status} for {url}"
    )));
  }
  Ok(response)
}

async fn fetch_remote_manifest(code: &str) -> Result<GaleManifest, ValheimModError> {
  let url = profile_url(code, true)?;
  let body = get(&url)
    .await?
    .text()
    .await
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
  let meta: GaleProfileMeta = serde_json::from_str(&body)
    .map_err(|e| ValheimModError::ManifestDeserializeError(format!("Gale profile meta: {e}")))?;
  Ok(meta.manifest)
}

/// Fetches the profile manifest for `code`, falling back to the last one fetched when Gale
/// cannot be reached.
pub async fn fetch_manifest(code: &str) -> Result<GaleManifest, ValheimModError> {
  let cache = cached_manifest_path(code);
  match fetch_remote_manifest(code).await {
    Ok(manifest) => {
      if let Some(parent) = cache.parent() {
        let _ = fs::create_dir_all(parent);
      }
      match serde_json::to_string_pretty(&manifest) {
        Ok(serialized) => {
          if let Err(e) = fs::write(&cache, serialized) {
            debug!("Could not cache Gale manifest at {}: {e}", cache.display());
          }
        }
        Err(e) => debug!("Could not serialize Gale manifest: {e}"),
      }
      Ok(manifest)
    }
    Err(e) => {
      let cached = fs::read_to_string(&cache)
        .ok()
        .and_then(|s| serde_json::from_str::<GaleManifest>(&s).ok());
      match cached {
        Some(manifest) => {
          warn!("Could not fetch Gale profile ({e}); using the last synced copy");
          Ok(manifest)
        }
        None => Err(e),
      }
    }
  }
}

/// Turns the enabled mods of a manifest into `MODS` entries (`ts:Author-Name-1.2.3`).
pub fn manifest_to_mod_entries(manifest: &GaleManifest) -> Vec<String> {
  manifest
    .mods
    .iter()
    .filter_map(|m| {
      if !m.enabled {
        debug!("Skipping disabled Gale mod {}", m.name);
        return None;
      }
      if m.name.ends_with(&format!("-{BEPINEX_PACK_NAME}")) {
        debug!("Skipping {}; BepInEx is installed by the container", m.name);
        return None;
      }
      let repository = match m.source.as_deref() {
        None => ModRepository::Thunderstore,
        Some(source) => match ModRepository::parse(source) {
          Some(repository) => repository,
          None => {
            warn!(
              "Skipping Gale mod {} from unsupported source {source:?}",
              m.name
            );
            return None;
          }
        },
      };
      let v = &m.version;
      Some(format!(
        "{}:{}-{}.{}.{}",
        repository.alias(),
        m.name,
        v.major,
        v.minor,
        v.patch
      ))
    })
    .collect()
}

/// `Author-Name` for a dependency-string entry (prefix and version stripped); `None` for URLs
/// and anything else that is not a dependency string.
fn package_key(entry: &str) -> Option<String> {
  let (_, rest) = split_repository_prefix(entry);
  parse_mod_string(rest).map(|(author, name, _)| format!("{author}-{name}").to_ascii_lowercase())
}

/// Profile entries first, then `MODS`. A `MODS` entry naming the same package as a profile
/// entry replaces it, so a version can be pinned without editing the Gale profile.
pub fn merge_mod_entries(profile: Vec<String>, mods: Vec<String>) -> Vec<String> {
  let overridden: HashSet<String> = mods.iter().filter_map(|m| package_key(m)).collect();
  let mut seen: HashSet<String> = HashSet::new();
  profile
    .into_iter()
    .filter(|entry| match package_key(entry) {
      Some(key) if overridden.contains(&key) => {
        info!("MODS overrides Gale profile entry {entry}");
        false
      }
      _ => true,
    })
    .chain(mods)
    .filter(|entry| seen.insert(entry.clone()))
    .collect()
}

/// Rejects zip paths that would escape the config directory.
fn safe_relative(path: &Path) -> Option<PathBuf> {
  let mut out = PathBuf::new();
  for component in path.components() {
    match component {
      Component::Normal(part) => out.push(part),
      Component::CurDir => {}
      _ => return None,
    }
  }
  (!out.as_os_str().is_empty()).then_some(out)
}

/// Copies every file under `BepInEx/config/` in the profile zip into `dest`, overwriting.
/// Returns the number of files written.
pub fn extract_configs(zip_bytes: &[u8], dest: &Path) -> Result<usize, ValheimModError> {
  let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
    .map_err(|e| ValheimModError::ZipArchiveError(e.to_string()))?;
  let mut written = 0;
  for i in 0..archive.len() {
    let mut file = archive
      .by_index(i)
      .map_err(|e| ValheimModError::ZipArchiveError(e.to_string()))?;
    if file.is_dir() {
      continue;
    }
    let name = file.name().replace('\\', "/");
    let Some(relative) = name
      .strip_prefix("BepInEx/config/")
      .and_then(|r| safe_relative(Path::new(r)))
    else {
      continue;
    };
    let target = dest.join(relative);
    if let Some(parent) = target.parent() {
      fs::create_dir_all(parent)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
    }
    let mut out =
      fs::File::create(&target).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
    std::io::copy(&mut file, &mut out)
      .map_err(|e| ValheimModError::ExtractionError(e.to_string()))?;
    written += 1;
  }
  Ok(written)
}

async fn sync_configs(code: &str) -> Result<(), ValheimModError> {
  let url = profile_url(code, false)?;
  let bytes = get(&url)
    .await?
    .bytes()
    .await
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
  let dest = PathBuf::from(bepinex_config_directory());
  let written = extract_configs(&bytes, &dest)?;
  info!(
    "Synced {written} config file(s) from the Gale profile into {}",
    dest.display()
  );
  Ok(())
}

/// Resolves `GALE_SYNC_CODE` into `MODS` entries and, when `GALE_SYNC_CONFIGS` is on, syncs
/// the profile's configs. Returns an empty list when no code is set.
pub async fn gale_mod_entries() -> Result<Vec<String>, ValheimModError> {
  let Some(code) = gale_sync_code() else {
    return Ok(vec![]);
  };

  let manifest = fetch_manifest(&code).await?;
  let entries = manifest_to_mod_entries(&manifest);
  info!(
    "Gale profile {} provides {} mod(s)",
    manifest.profile_name.as_deref().unwrap_or(&code),
    entries.len()
  );

  if is_env_var_truthy_with_default(GALE_SYNC_CONFIGS_VAR, false) {
    // Configs are a convenience; a failure here should not keep the server down.
    if let Err(e) = sync_configs(&code).await {
      warn!("Failed to sync Gale profile configs: {e}");
    }
  }

  Ok(entries)
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockito::Server;
  use serial_test::serial;
  use std::env;
  use std::io::Write;

  const META: &str = r#"{
    "id": "abc123",
    "createdAt": "2026-09-01T00:00:00Z",
    "updatedAt": "2026-09-02T00:00:00Z",
    "owner": { "name": "someone", "displayName": "Someone", "avatar": null },
    "manifest": {
      "profileName": "Valheim-1.0-Pack",
      "community": "valheim",
      "mods": [
        { "name": "denikson-BepInExPack_Valheim", "version": { "major": 5, "minor": 4, "patch": 2350 }, "enabled": true, "source": "Thunderstore" },
        { "name": "Advize-PlantEasily", "version": { "major": 2, "minor": 2, "patch": 0 }, "enabled": true, "source": "Thunderstore" },
        { "name": "ValheimModding-YamlDotNet", "version": { "major": 16, "minor": 3, "patch": 1 }, "enabled": true, "source": "Hexium" },
        { "name": "ZenDragon-ZenDistributor", "version": { "major": 1, "minor": 3, "patch": 0 }, "enabled": false, "source": "Thunderstore" },
        { "name": "Old-Profile", "versionNumber": { "major": 1, "minor": 0, "patch": 0 }, "enabled": true }
      ]
    }
  }"#;

  fn manifest() -> GaleManifest {
    serde_json::from_str::<GaleProfileMeta>(META)
      .unwrap()
      .manifest
  }

  #[test]
  fn converts_enabled_mods_to_prefixed_entries() {
    assert_eq!(
      manifest_to_mod_entries(&manifest()),
      vec![
        "ts:Advize-PlantEasily-2.2.0",
        "hex:ValheimModding-YamlDotNet-16.3.1",
        "ts:Old-Profile-1.0.0",
      ]
    );
  }

  #[test]
  fn mods_entries_are_appended_and_override_profile_versions() {
    let merged = merge_mod_entries(
      vec![
        "ts:Advize-PlantEasily-2.2.0".into(),
        "hex:ValheimModding-YamlDotNet-16.3.1".into(),
      ],
      vec![
        "Advize-PlantEasily-2.3.0".into(),
        "https://example.com/ServerOnly.dll".into(),
        "hex:ValheimModding-YamlDotNet-16.3.1".into(),
      ],
    );
    assert_eq!(
      merged,
      vec![
        "Advize-PlantEasily-2.3.0",
        "https://example.com/ServerOnly.dll",
        "hex:ValheimModding-YamlDotNet-16.3.1",
      ]
    );
  }

  #[test]
  fn profile_url_encodes_the_code() {
    env::remove_var(GALE_SYNC_URL_VAR);
    assert_eq!(
      profile_url("ab/c+d==", true).unwrap().as_str(),
      "https://gale.kesomannen.com/api/profile/ab%2Fc+d==/meta"
    );
    assert_eq!(
      profile_url("abc123", false).unwrap().as_str(),
      "https://gale.kesomannen.com/api/profile/abc123"
    );
  }

  #[test]
  fn extracts_only_bepinex_configs() {
    let mut buf = Vec::new();
    {
      let mut zipw = zip::ZipWriter::new(Cursor::new(&mut buf));
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      for (name, body) in [
        ("export.r2x", "profileName: x"),
        ("BepInEx/config/advize.PlantEasily.cfg", "a = 1"),
        ("BepInEx/config/sub/nested.yml", "b: 2"),
        ("BepInEx/config/../../escape.cfg", "nope"),
        ("BepInEx/plugins/Some.dll", "nope"),
      ] {
        zipw.start_file(name, options).unwrap();
        zipw.write_all(body.as_bytes()).unwrap();
      }
      zipw.finish().unwrap();
    }

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("config");
    assert_eq!(extract_configs(&buf, &dest).unwrap(), 2);
    assert_eq!(
      fs::read_to_string(dest.join("advize.PlantEasily.cfg")).unwrap(),
      "a = 1"
    );
    assert!(dest.join("sub/nested.yml").exists());
    assert!(!dir.path().join("escape.cfg").exists());
  }

  #[tokio::test]
  #[serial]
  async fn falls_back_to_cached_manifest_when_gale_is_down() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, format!("{}/api", server.url()));

    let ok = server
      .mock("GET", "/api/profile/abc123/meta")
      .with_status(200)
      .with_body(META)
      .create_async()
      .await;
    assert_eq!(fetch_manifest("abc123").await.unwrap().mods.len(), 5);
    ok.remove_async().await;

    let _down = server
      .mock("GET", "/api/profile/abc123/meta")
      .with_status(404)
      .create_async()
      .await;
    let cached = fetch_manifest("abc123").await.unwrap();
    assert_eq!(cached.profile_name.as_deref(), Some("Valheim-1.0-Pack"));
    assert!(fetch_manifest("other1").await.is_err());

    env::remove_var(GALE_SYNC_URL_VAR);
    env::remove_var("GAME_LOCATION");
  }
}

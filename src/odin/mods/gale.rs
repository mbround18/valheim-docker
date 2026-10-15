//! Gale profile sync (<https://github.com/Kesomannen/gale/wiki/Profile-sync>).
//!
//! A Gale user can publish a mod profile and hand out its sync code. `GALE_SYNC_CODE` makes the
//! server install exactly the mods (and versions) of that profile, so clients who pull the same
//! code never hit a version mismatch. The profile is read without authentication:
//!
//! `GET {GALE_SYNC_URL}/profile/{code}` returns a ZIP whose `export.r2x` YAML manifest
//! carries authoritative package sources, alongside optional `BepInEx/config` files.
//!
//! Enabled mods become internal store-preference entries that are merged ahead of `MODS`, so `MODS` can still
//! add server-only mods or pin a different version of a profile mod. `GALE_SYNC_CONFIGS=true`
//! also copies the profile's `BepInEx/config` files over the server's.

use crate::errors::ValheimModError;
use crate::utils::common_paths::{bepinex_config_directory, mods_staging_directory};
use crate::utils::environment::{fetch_var, is_env_var_truthy_with_default};
use crate::utils::http_pool::HttpPool;
use crate::utils::thunderstore_http::{send_with_pacing, Pacing};
use crate::utils::{get_md5_hash, parse_mod_string, split_repository_prefix, ModRepository};
use indexmap::IndexSet;
use log::{debug, info, warn};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

pub const GALE_SYNC_CODE_VAR: &str = "GALE_SYNC_CODE";
pub const GALE_SYNC_CONFIGS_VAR: &str = "GALE_SYNC_CONFIGS";
const GALE_SYNC_URL_VAR: &str = "GALE_SYNC_URL";
const DEFAULT_GALE_SYNC_URL: &str = "https://gale.kesomannen.com/api";

/// The container installs `BepInEx` itself (`TYPE=BepInEx`), so the pack entry is skipped.
const BEPINEX_PACK_NAME: &str = "BepInExPack_Valheim";

/// Largest profile zip accepted, compressed. A profile carries config text, so real ones
/// are orders of magnitude under this; the cap is here so a hostile or truncated response
/// cannot be read into memory without bound.
const MAX_PROFILE_ZIP_BYTES: u64 = 64 * 1024 * 1024;

/// Largest total uncompressed payload written by one config sync, which is what bounds
/// zip-bomb expansion: the compressed cap above says nothing about what it inflates to.
const MAX_CONFIG_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

/// Bound the expanded YAML separately from config files.
const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

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
  #[serde(default)]
  pub community: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CachedSnapshot {
  schema_version: u32,
  manifest: GaleManifest,
}

struct GaleProfile {
  manifest: GaleManifest,
  zip_bytes: Option<Vec<u8>>,
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

/// `{base}/profile/{code}`, with the code percent-encoded as a single path segment
/// (base64 codes may contain `/` or `+`).
fn profile_url(code: &str) -> Result<Url, ValheimModError> {
  let mut url = Url::parse(&gale_base_url()).map_err(|_| ValheimModError::InvalidUrl)?;
  {
    let mut segments = url
      .path_segments_mut()
      .map_err(|()| ValheimModError::InvalidUrl)?;
    segments.pop_if_empty().push("profile").push(code);
  }
  Ok(url)
}

/// A separate namespace excludes old source-less metadata. Include the API URL and exact
/// code in the identity so different codes and self-hosted instances cannot share snapshots.
fn cached_manifest_path(code: &str) -> PathBuf {
  let identity = get_md5_hash(&format!("{}/{code}", gale_base_url()));
  PathBuf::from(mods_staging_directory()).join(format!("gale-zip-v1-{identity}.json"))
}

fn check_status(
  response: reqwest::Response,
  url: &Url,
) -> Result<reqwest::Response, ValheimModError> {
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

fn invalid_manifest(message: impl Into<String>) -> ValheimModError {
  ValheimModError::ManifestDeserializeError(format!("Gale export.r2x: {}", message.into()))
}

/// Validate the complete snapshot before it can cause any reconciliation or be cached.
fn validate_manifest(
  manifest: &mut GaleManifest,
  legacy_source_default: bool,
) -> Result<(), ValheimModError> {
  if manifest
    .community
    .as_deref()
    .is_some_and(|c| c != "valheim")
  {
    return Err(invalid_manifest("community must be valheim"));
  }
  let mut packages = HashSet::new();
  for item in &mut manifest.mods {
    let version = &item.version;
    let entry = format!(
      "{}-{}.{}.{}",
      item.name, version.major, version.minor, version.patch
    );
    let Some((author, name, _)) = parse_mod_string(&entry) else {
      return Err(invalid_manifest(format!(
        "invalid package name {:?}",
        item.name
      )));
    };
    if ![author, name]
      .iter()
      .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
      return Err(invalid_manifest(format!(
        "invalid package name {:?}",
        item.name
      )));
    }
    if !packages.insert(item.name.to_ascii_lowercase()) {
      return Err(invalid_manifest(format!("duplicate package {}", item.name)));
    }
    let repository = match item.source.as_deref() {
      None if legacy_source_default => ModRepository::Thunderstore,
      Some(source) => ModRepository::parse(source)
        .ok_or_else(|| invalid_manifest(format!("unsupported source {source:?}")))?,
      None => {
        return Err(invalid_manifest(
          "cached snapshot is missing a validated source",
        ))
      }
    };
    // Persist every validated source, including the legacy Thunderstore default.
    item.source = Some(repository.to_string());
  }
  Ok(())
}

/// Zip 8 deduplicates raw names before exposing entries. Walk the original central
/// directory to reject conflicting manifests before its last-entry selection can hide one.
/// This only inspects bounded slices of the downloaded ZIP, without inflating any files.
fn validate_raw_manifest_count(bytes: &[u8], directory_start: u64) -> Result<(), ValheimModError> {
  let malformed = || invalid_manifest("truncated profile ZIP central directory");
  let offset = usize::try_from(directory_start).map_err(|_| malformed())?;
  let mut directory = bytes.get(offset..).ok_or_else(malformed)?;
  let mut manifests = 0;
  while directory.starts_with(b"PK\x01\x02") {
    // APPNOTE central directory: 46 fixed bytes, then name, extra data and comment.
    let header = directory.get(..46).ok_or_else(malformed)?;
    let name_length = u16::from_le_bytes([header[28], header[29]]) as usize;
    let extra_length = u16::from_le_bytes([header[30], header[31]]) as usize;
    let comment_length = u16::from_le_bytes([header[32], header[33]]) as usize;
    // Each length is a u16, so this sum fits usize on supported targets.
    let entry_length = 46 + name_length + extra_length + comment_length;
    let entry = directory.get(..entry_length).ok_or_else(malformed)?;
    if &entry[46..46 + name_length] == b"export.r2x" {
      manifests += 1;
      if manifests > 1 {
        return Err(invalid_manifest(
          "profile ZIP must contain exactly one export.r2x",
        ));
      }
    }
    directory = &directory[entry_length..];
  }
  if manifests != 1 {
    return Err(invalid_manifest(
      "profile ZIP must contain exactly one export.r2x",
    ));
  }
  Ok(())
}

fn manifest_from_zip(
  bytes: &[u8],
  max_manifest_bytes: u64,
) -> Result<GaleManifest, ValheimModError> {
  let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
    .map_err(|e| invalid_manifest(format!("invalid profile ZIP: {e}")))?;
  validate_raw_manifest_count(bytes, archive.central_directory_start())?;
  if archive
    .file_names()
    .filter(|name| *name == "export.r2x")
    .count()
    != 1
  {
    return Err(invalid_manifest(
      "profile ZIP must contain exactly one export.r2x",
    ));
  }
  let mut file = archive
    .by_name("export.r2x")
    .map_err(|e| invalid_manifest(e.to_string()))?;
  if file.size() > max_manifest_bytes {
    return Err(invalid_manifest(format!(
      "manifest exceeds {max_manifest_bytes} bytes"
    )));
  }
  let mut yaml = Vec::new();
  file
    .by_ref()
    .take(max_manifest_bytes.saturating_add(1))
    .read_to_end(&mut yaml)
    .map_err(|e| invalid_manifest(e.to_string()))?;
  if yaml.len() as u64 > max_manifest_bytes {
    return Err(invalid_manifest(format!(
      "manifest exceeds {max_manifest_bytes} bytes"
    )));
  }
  let mut manifest: GaleManifest =
    serde_saphyr::from_slice(&yaml).map_err(|e| invalid_manifest(e.to_string()))?;
  validate_manifest(&mut manifest, true)?;
  Ok(manifest)
}

async fn fetch_profile(code: &str) -> Result<GaleProfile, ValheimModError> {
  let cache = cached_manifest_path(code);
  let bytes = match download_profile_zip(&profile_url(code)?).await {
    Ok(bytes) => bytes,
    Err(error @ ValheimModError::DownloadError(_)) => {
      // Read bounded, versioned snapshots only; never trust the old /meta cache.
      let cached = fs::File::open(&cache).ok().and_then(|file| {
        let mut bytes = Vec::new();
        file
          .take(MAX_MANIFEST_BYTES + 1)
          .read_to_end(&mut bytes)
          .ok()?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
          return None;
        }
        let mut snapshot: CachedSnapshot = serde_json::from_slice(&bytes).ok()?;
        if snapshot.schema_version != 1 {
          return None;
        }
        validate_manifest(&mut snapshot.manifest, false).ok()?;
        Some(snapshot.manifest)
      });
      if let Some(manifest) = cached {
        warn!("Could not fetch Gale profile ({error}); using the last synced copy");
        return Ok(GaleProfile {
          manifest,
          zip_bytes: None,
        });
      }
      return Err(error);
    }
    Err(error) => return Err(error),
  };
  let manifest = manifest_from_zip(&bytes, MAX_MANIFEST_BYTES)?;
  let snapshot = CachedSnapshot {
    schema_version: 1,
    manifest: manifest.clone(),
  };
  // Write beside the destination then rename, preserving the last complete snapshot.
  let write_cache = || -> Result<(), Box<dyn std::error::Error>> {
    let parent = cache.parent().ok_or("missing cache parent")?;
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(staged.as_file_mut(), &snapshot)?;
    staged.persist(&cache)?;
    Ok(())
  };
  if let Err(error) = write_cache() {
    debug!("Could not cache Gale profile: {error}");
  }
  Ok(GaleProfile {
    manifest,
    zip_bytes: Some(bytes),
  })
}

/// Test convenience for fetching the manifest without optional config extraction.
#[cfg(test)]
async fn fetch_manifest(code: &str) -> Result<GaleManifest, ValheimModError> {
  Ok(fetch_profile(code).await?.manifest)
}

/// Turns the enabled mods of a manifest into `MODS` entries.
///
/// Internal `gale-ts:`/`gale-hex:` prefixes express a preference with exact-version
/// absence-only fallback. Legacy missing sources prefer Thunderstore, as in Gale.
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
      let suffix = format!("{}-{}.{}.{}", m.name, v.major, v.minor, v.patch);
      Some(format!("gale-{}:{suffix}", repository.alias()))
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
/// Without a profile, `MODS` passes through untouched.
pub fn merge_mod_entries(profile: Vec<String>, mods: Vec<String>) -> Vec<String> {
  if profile.is_empty() {
    return mods;
  }
  let overridden: HashSet<String> = mods.iter().filter_map(|m| package_key(m)).collect();
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
    .collect::<IndexSet<String>>()
    .into_iter()
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
  extract_configs_capped(zip_bytes, dest, MAX_CONFIG_TOTAL_BYTES)
}

/// [`extract_configs`] with an explicit budget, so the cap itself can be tested without
/// building a multi-hundred-megabyte archive.
fn extract_configs_capped(
  zip_bytes: &[u8],
  dest: &Path,
  max_total_bytes: u64,
) -> Result<usize, ValheimModError> {
  let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
    .map_err(|e| ValheimModError::ZipArchiveError(e.to_string()))?;

  // Entries are unpacked into a staging directory and only moved into `dest` once the
  // whole archive has been read. Refusing partway through - the budget, a corrupt entry -
  // then leaves the server's live configs exactly as they were rather than half synced.
  // Staged beside `dest` so the moves below are same-filesystem renames, and dropped on
  // every exit path, so a refusal cleans up after itself.
  let staging_parent = match dest.parent() {
    Some(parent) if !parent.as_os_str().is_empty() => parent,
    _ => dest,
  };
  fs::create_dir_all(staging_parent)
    .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
  let staging = tempfile::Builder::new()
    .prefix(".gale-config-sync-")
    .tempdir_in(staging_parent)
    .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;

  let mut staged: Vec<PathBuf> = Vec::new();
  let mut total: u64 = 0;
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
    let target = staging.path().join(&relative);
    if let Some(parent) = target.parent() {
      fs::create_dir_all(parent)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
    }
    let mut out =
      fs::File::create(&target).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
    // Read one byte past what the budget allows: if that byte materialises the archive
    // inflates beyond the cap, so stop rather than keep writing.
    let remaining = max_total_bytes.saturating_sub(total);
    let copied = std::io::copy(
      &mut file.by_ref().take(remaining.saturating_add(1)),
      &mut out,
    )
    .map_err(|e| ValheimModError::ExtractionError(e.to_string()))?;
    if copied > remaining {
      return Err(ValheimModError::ExtractionError(format!(
        "Gale profile configs expand beyond {max_total_bytes} bytes; refusing to continue"
      )));
    }
    total += copied;
    staged.push(relative);
  }

  // The archive is whole and within budget, so the staged tree can replace what is live.
  for relative in &staged {
    let target = dest.join(relative);
    if let Some(parent) = target.parent() {
      fs::create_dir_all(parent)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
    }
    move_into_place(&staging.path().join(relative), &target)?;
  }
  Ok(staged.len())
}

/// Moves a staged config into place, copying instead when the two sit on different
/// filesystems.
///
/// Staging lives beside `BepInEx/config` so these are normally same-filesystem renames,
/// but the config directory is sometimes its own bind mount or volume, and `rename` cannot
/// cross a mount boundary - it fails with `EXDEV`. Falling back to a copy keeps config sync
/// working there. Per file that is less atomic than a rename, but the whole archive has
/// already been read and checked against the budget by this point, which is what staging is
/// for; the alternative is the feature failing outright on such a layout.
fn move_into_place(from: &Path, to: &Path) -> Result<(), ValheimModError> {
  if fs::rename(from, to).is_ok() {
    return Ok(());
  }
  copy_into_place(from, to)
}

/// The copy fallback for [`move_into_place`], split out so it can be tested directly
/// rather than only on a host that happens to straddle two filesystems.
fn copy_into_place(from: &Path, to: &Path) -> Result<(), ValheimModError> {
  fs::copy(from, to).map_err(|e| ValheimModError::ExtractionError(e.to_string()))?;
  // Best effort: the staging directory is removed wholesale when it drops.
  let _ = fs::remove_file(from);
  Ok(())
}

/// Fetches the profile zip, following the redirect off the Gale API and refusing a body
/// larger than [`MAX_PROFILE_ZIP_BYTES`].
async fn download_profile_zip(url: &Url) -> Result<Vec<u8>, ValheimModError> {
  download_profile_zip_capped(url, MAX_PROFILE_ZIP_BYTES).await
}

async fn download_profile_zip_capped(
  url: &Url,
  max_bytes: u64,
) -> Result<Vec<u8>, ValheimModError> {
  // The zip redirects to the mod CDN and the pooled client does not follow redirects, so
  // this goes through the redirect walker. Gale is not a mod repository host, so pacing is
  // requested explicitly - otherwise the walker would send it direct, giving up the
  // backoff and 429 handling every other download gets.
  let request = HttpPool::global().client().get(url.clone());
  let response = send_with_pacing(request, url.as_str(), "gale", Pacing::Always).await?;
  let mut response = check_status(response, url)?;

  let too_large = || {
    ValheimModError::ManifestDeserializeError(format!(
      "Gale profile zip is larger than {max_bytes} bytes"
    ))
  };
  // A declared length lets us bail before transferring anything; it is a hint, so the
  // loop below still enforces the cap on what actually arrives.
  if response.content_length().is_some_and(|l| l > max_bytes) {
    return Err(too_large());
  }

  let mut bytes: Vec<u8> = Vec::new();
  while let Some(chunk) = response
    .chunk()
    .await
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?
  {
    if bytes.len() as u64 + chunk.len() as u64 > max_bytes {
      return Err(too_large());
    }
    bytes.extend_from_slice(&chunk);
  }
  Ok(bytes)
}

fn sync_configs(bytes: &[u8]) -> Result<(), ValheimModError> {
  let dest = PathBuf::from(bepinex_config_directory());
  let written = extract_configs(bytes, &dest)?;
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

  let profile = fetch_profile(&code).await?;
  let manifest = profile.manifest;
  let entries = manifest_to_mod_entries(&manifest);
  info!(
    "Gale profile {} provides {} mod(s)",
    manifest.profile_name.as_deref().unwrap_or(&code),
    entries.len()
  );

  if is_env_var_truthy_with_default(GALE_SYNC_CONFIGS_VAR, false) {
    // Configs are a convenience; a failure here should not keep the server down.
    match profile.zip_bytes {
      Some(bytes) => {
        if let Err(e) = sync_configs(&bytes) {
          warn!("Failed to sync Gale profile configs: {e}");
        }
      }
      None => warn!("Gale is unavailable; keeping existing profile configs"),
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

  #[derive(Deserialize)]
  struct GaleProfileMeta {
    manifest: GaleManifest,
  }

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

  /// Shape copied from a real Gale profile where `source` is omitted and repository must be
  /// defaulted to Thunderstore by Gale when read from export.r2x.
  const META_WITHOUT_SOURCES: &str = r#"{
    "id": "9M5Z3V",
    "createdAt": "2026-09-19T00:00:00Z",
    "updatedAt": "2026-09-19T00:00:00Z",
    "owner": { "name": "someone", "displayName": "Someone", "avatar": null },
    "manifest": {
      "profileName": "Mixed-Repo-Pack",
      "community": "valheim",
      "mods": [
        { "name": "Azumatt-AzuCraftyBoxes", "version": { "major": 1, "minor": 8, "patch": 22 }, "enabled": true },
        { "name": "Advize-PlantEverything", "version": { "major": 1, "minor": 21, "patch": 2 }, "enabled": true },
        { "name": "Disabled-TestMod", "version": { "major": 9, "minor": 9, "patch": 9 }, "enabled": false }
      ]
    }
  }"#;

  fn manifest() -> GaleManifest {
    serde_json::from_str::<GaleProfileMeta>(META)
      .unwrap()
      .manifest
  }

  #[tokio::test]
  #[serial]
  async fn zip_manifest_is_authoritative_with_configs_disabled() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    env::set_var(GALE_SYNC_CODE_VAR, "zip-source");
    env::set_var(GALE_SYNC_CONFIGS_VAR, "false");
    env::set_var("MODS_REPOSITORY", "thunderstore");
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, server.url());
    let mut bytes = Vec::new();
    {
      let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      zip.start_file("export.r2x", options).unwrap();
      zip.write_all(b"profileName: ZIP\ncommunity: valheim\nmods:\n- name: Author-Mod\n  version: {major: 1, minor: 0, patch: 0}\n  enabled: true\n  source: Hexium\n").unwrap();
      zip.finish().unwrap();
    }
    let archive = server
      .mock("GET", "/profile/zip-source")
      .with_body(bytes)
      .expect(1)
      .create_async()
      .await;
    let meta = server
      .mock("GET", "/profile/zip-source/meta")
      .with_body(META_WITHOUT_SOURCES)
      .expect(0)
      .create_async()
      .await;
    assert_eq!(
      gale_mod_entries().await.unwrap(),
      vec!["gale-hex:Author-Mod-1.0.0"]
    );
    archive.assert_async().await;
    meta.assert_async().await;
    for key in [
      "GAME_LOCATION",
      GALE_SYNC_CODE_VAR,
      GALE_SYNC_CONFIGS_VAR,
      GALE_SYNC_URL_VAR,
      "MODS_REPOSITORY",
    ] {
      env::remove_var(key);
    }
  }

  fn profile_zip(manifest: &GaleManifest) -> Vec<u8> {
    zip_entries(&[("export.r2x", &serde_saphyr::to_string(manifest).unwrap())])
  }

  fn zip_entries(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
      let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      for (name, contents) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
      }
      writer.finish().unwrap();
    }
    bytes
  }

  #[test]
  fn converts_enabled_mods_to_prefixed_entries() {
    assert_eq!(
      manifest_to_mod_entries(&manifest()),
      vec![
        "gale-ts:Advize-PlantEasily-2.2.0",
        "gale-hex:ValheimModding-YamlDotNet-16.3.1",
        "gale-ts:Old-Profile-1.0.0",
      ]
    );
  }

  #[test]
  fn mods_entries_are_appended_and_override_profile_versions() {
    let merged = merge_mod_entries(
      vec![
        "gale-ts:Advize-PlantEasily-2.2.0".into(),
        "gale-hex:ValheimModding-YamlDotNet-16.3.1".into(),
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
  fn mods_pass_through_untouched_without_a_profile() {
    let mods: Vec<String> = vec![
      "Advize-PlantEasily-2.3.0".into(),
      "Advize-PlantEasily-2.3.0".into(),
      "hex:ValheimModding-Jotunn-*".into(),
      "https://example.com/ServerOnly.dll".into(),
    ];
    assert_eq!(merge_mod_entries(vec![], mods.clone()), mods);
  }

  #[tokio::test]
  #[serial]
  async fn no_sync_code_means_no_entries_and_no_requests() {
    env::remove_var(GALE_SYNC_CODE_VAR);
    // An unroutable base URL: any request attempt would fail the test.
    env::set_var(GALE_SYNC_URL_VAR, "http://127.0.0.1:9/api");
    assert!(gale_mod_entries().await.unwrap().is_empty());
    env::set_var(GALE_SYNC_CODE_VAR, "   ");
    assert!(gale_mod_entries().await.unwrap().is_empty());
    env::remove_var(GALE_SYNC_CODE_VAR);
    env::remove_var(GALE_SYNC_URL_VAR);
  }

  #[test]
  #[serial]
  fn profile_url_encodes_the_code() {
    env::remove_var(GALE_SYNC_URL_VAR);
    assert_eq!(
      profile_url("ab/c+d==").unwrap().as_str(),
      "https://gale.kesomannen.com/api/profile/ab%2Fc+d=="
    );
    assert_eq!(
      profile_url("abc123").unwrap().as_str(),
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

  /// A profile that inflates past the budget must be refused, not written out. Built with
  /// a small explicit cap so the test does not need a real zip bomb.
  #[test]
  fn refuses_configs_that_expand_past_the_cap() {
    let mut buf = Vec::new();
    {
      let mut zipw = zip::ZipWriter::new(Cursor::new(&mut buf));
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      // An entry that fits precedes the one that blows the budget, so a partial write
      // would be visible as `first.cfg` landing in dest.
      zipw
        .start_file("BepInEx/config/first.cfg", options)
        .unwrap();
      zipw.write_all(&vec![b'a'; 512]).unwrap();
      zipw.start_file("BepInEx/config/bomb.cfg", options).unwrap();
      zipw.write_all(&vec![b'a'; 4096]).unwrap();
      zipw.finish().unwrap();
    }

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("config");
    // A config already on the server, to prove a refused sync leaves the live tree alone.
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("existing.cfg"), "keep me").unwrap();

    let err = extract_configs_capped(&buf, &dest, 1024).expect_err("cap must be enforced");
    assert!(
      matches!(err, ValheimModError::ExtractionError(_)),
      "unexpected error: {err:?}"
    );
    assert!(
      !dest.join("bomb.cfg").exists(),
      "the over-budget file must not be left behind"
    );
    assert!(
      !dest.join("first.cfg").exists(),
      "a refusal must not leave earlier entries half applied"
    );
    assert_eq!(
      fs::read_to_string(dest.join("existing.cfg")).unwrap(),
      "keep me",
      "a refused sync must not disturb the live configs"
    );
    assert!(
      !fs::read_dir(dir.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".gale-config-sync-")),
      "the staging directory must not be left behind"
    );

    // The same archive is fine when the budget accommodates it.
    assert_eq!(extract_configs_capped(&buf, &dest, 8192).unwrap(), 2);
    assert!(dest.join("bomb.cfg").exists());
    assert!(dest.join("first.cfg").exists());
  }

  /// `BepInEx/config` is sometimes its own mount, where `rename` out of staging fails with
  /// `EXDEV`. The copy fallback has to land the file and clear the staged copy, so config
  /// sync keeps working on that layout.
  #[test]
  fn copy_fallback_lands_the_file_and_clears_staging() {
    let dir = tempfile::tempdir().unwrap();
    let staged = dir.path().join("staged.cfg");
    let target = dir.path().join("config").join("landed.cfg");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&staged, "value = 1").unwrap();

    copy_into_place(&staged, &target).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "value = 1");
    assert!(!staged.exists(), "the staged copy must be cleared");

    // Overwriting an existing config is the documented behaviour of a sync.
    let next = dir.path().join("next.cfg");
    fs::write(&next, "value = 2").unwrap();
    copy_into_place(&next, &target).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "value = 2");
  }

  /// `move_into_place` takes the rename when both sides share a filesystem, which is the
  /// normal layout, and is what the fallback above stands in for when they do not.
  #[test]
  fn move_into_place_handles_the_same_filesystem_case() {
    let dir = tempfile::tempdir().unwrap();
    let staged = dir.path().join("staged.cfg");
    let target = dir.path().join("config").join("landed.cfg");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&staged, "value = 3").unwrap();

    move_into_place(&staged, &target).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "value = 3");
    assert!(!staged.exists());
  }

  #[tokio::test]
  #[serial]
  async fn falls_back_to_cached_manifest_when_gale_is_down() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, format!("{}/api", server.url()));

    let ok = server
      .mock("GET", "/api/profile/abc123")
      .with_status(200)
      .with_body(profile_zip(&manifest()))
      .create_async()
      .await;
    assert_eq!(fetch_manifest("abc123").await.unwrap().mods.len(), 5);
    ok.remove_async().await;

    let _down = server
      .mock("GET", "/api/profile/abc123")
      .with_status(404)
      .create_async()
      .await;
    let cached = fetch_manifest("abc123").await.unwrap();
    assert_eq!(cached.profile_name.as_deref(), Some("Valheim-1.0-Pack"));
    assert!(fetch_manifest("other1").await.is_err());

    env::remove_var(GALE_SYNC_URL_VAR);
    env::remove_var("GAME_LOCATION");
  }

  #[tokio::test]
  #[serial]
  async fn legacy_zip_sources_default_to_thunderstore() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, format!("{}/api", server.url()));
    let _meta = server
      .mock("GET", "/api/profile/9M5Z3V")
      .with_status(200)
      .with_body(profile_zip(
        &serde_json::from_str::<GaleProfileMeta>(META_WITHOUT_SOURCES)
          .unwrap()
          .manifest,
      ))
      .create_async()
      .await;

    let manifest = fetch_manifest("9M5Z3V").await.unwrap();
    let entries = manifest_to_mod_entries(&manifest);
    assert_eq!(
      entries,
      vec![
        "gale-ts:Azumatt-AzuCraftyBoxes-1.8.22",
        "gale-ts:Advize-PlantEverything-1.21.2",
      ]
    );

    env::remove_var(GALE_SYNC_URL_VAR);
    env::remove_var("GAME_LOCATION");
  }
  /// Produced with Python's zipfile writer, which preserves duplicate raw names;
  /// zip 8's regular Rust writer rejects them and its reader silently keeps the last.
  #[test]
  fn rejects_conflicting_raw_duplicate_manifests_before_accepting_empty_profile() {
    let bytes = include_bytes!("../tests/resources/gale.duplicate-manifest.zip");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(
      archive
        .file_names()
        .filter(|name| *name == "export.r2x")
        .count(),
      1
    );
    let mut selected = String::new();
    archive
      .by_name("export.r2x")
      .unwrap()
      .read_to_string(&mut selected)
      .unwrap();
    assert_eq!(
      selected, "mods: []\n",
      "fixture must expose the lossy last-entry selection"
    );
    let error = manifest_from_zip(bytes, MAX_MANIFEST_BYTES)
      .expect_err("ambiguous ZIP must never reconcile to an empty profile");
    assert!(error.to_string().contains("exactly one export.r2x"));
  }

  #[test]
  fn raw_manifest_scan_bounds_directory_offsets_headers_and_variable_lengths() {
    let mut oversized_name = vec![0; 46];
    oversized_name[..4].copy_from_slice(b"PK\x01\x02");
    oversized_name[28..30].copy_from_slice(&u16::MAX.to_le_bytes());
    for (bytes, offset) in [
      (b"PK\x01\x02".to_vec(), 0),
      (oversized_name, 0),
      (vec![], u64::MAX),
    ] {
      let error = validate_raw_manifest_count(&bytes, offset).unwrap_err();
      assert!(error
        .to_string()
        .contains("truncated profile ZIP central directory"));
    }
  }

  #[test]
  fn rejects_invalid_or_missing_zip_manifests() {
    for bytes in [
      b"not a zip".to_vec(),
      zip_entries(&[("other.yml", "mods: []")]),
      zip_entries(&[("export.r2x", "mods: [")]),
      zip_entries(&[("export.r2x", "profileName: missing mods")]),
    ] {
      assert!(matches!(
        manifest_from_zip(&bytes, MAX_MANIFEST_BYTES),
        Err(ValheimModError::ManifestDeserializeError(_))
      ));
    }
  }

  #[test]
  fn validates_all_sources_names_duplicates_and_community_before_accepting() {
    for yaml in [
      "community: othergame\nmods: []".to_string(),
      "mods:\n- name: Author-Mod\n  source: OtherStore\n  version: {major: 1, minor: 0, patch: 0}".to_string(),
      "mods:\n- name: ../Author-Mod\n  version: {major: 1, minor: 0, patch: 0}".to_string(),
      "mods:\n- name: Author-Mod\n  version: {major: 1, minor: 0, patch: 0}\n- name: AUTHOR-mod\n  version: {major: 2, minor: 0, patch: 0}".to_string(),
      "mods:\n- name: Author-Mod\n  enabled: false\n  source: OtherStore\n  version: {major: 1, minor: 0, patch: 0}".to_string(),
    ] {
      assert!(matches!(manifest_from_zip(&zip_entries(&[("export.r2x", &yaml)]), MAX_MANIFEST_BYTES), Err(ValheimModError::ManifestDeserializeError(_))), "accepted {yaml}");
    }
    let empty = manifest_from_zip(
      &zip_entries(&[("export.r2x", "mods: []")]),
      MAX_MANIFEST_BYTES,
    )
    .unwrap();
    assert!(manifest_to_mod_entries(&empty).is_empty());
  }

  #[test]
  fn expanded_manifest_limit_accepts_boundary_and_rejects_next_byte() {
    let yaml = "profileName: Boundary\nmods: []\n";
    let bytes = zip_entries(&[("export.r2x", yaml)]);
    assert_eq!(
      manifest_from_zip(&bytes, yaml.len() as u64)
        .unwrap()
        .profile_name
        .as_deref(),
      Some("Boundary")
    );
    let error = manifest_from_zip(&bytes, yaml.len() as u64 - 1).unwrap_err();
    assert!(error.to_string().contains("manifest exceeds"));
  }

  #[tokio::test]
  #[serial]
  async fn compressed_profile_limit_bounds_declared_and_streamed_bodies() {
    let mut server = Server::new_async().await;
    let declared = server
      .mock("GET", "/declared")
      .with_body("12345")
      .expect(1)
      .create_async()
      .await;
    let streamed = server
      .mock("GET", "/streamed")
      .with_chunked_body(|writer| writer.write_all(b"12345"))
      .expect(1)
      .create_async()
      .await;
    let boundary = server
      .mock("GET", "/boundary")
      .with_body("1234")
      .expect(1)
      .create_async()
      .await;
    for path in ["declared", "streamed"] {
      let error =
        download_profile_zip_capped(&Url::parse(&format!("{}/{path}", server.url())).unwrap(), 4)
          .await
          .unwrap_err();
      assert!(matches!(
        error,
        ValheimModError::ManifestDeserializeError(_)
      ));
      assert!(error.to_string().contains("larger than 4 bytes"));
    }
    assert_eq!(
      download_profile_zip_capped(
        &Url::parse(&format!("{}/boundary", server.url())).unwrap(),
        4
      )
      .await
      .unwrap(),
      b"1234"
    );
    declared.assert_async().await;
    streamed.assert_async().await;
    boundary.assert_async().await;
  }

  #[tokio::test]
  #[serial]
  async fn config_sync_reuses_the_authoritative_zip_download() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    env::set_var(GALE_SYNC_CODE_VAR, "configs");
    env::set_var(GALE_SYNC_CONFIGS_VAR, "true");
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, server.url());
    let yaml = serde_saphyr::to_string(&manifest()).unwrap();
    let zip = server
      .mock("GET", "/profile/configs")
      .with_body(zip_entries(&[
        ("export.r2x", &yaml),
        ("BepInEx/config/test.cfg", "from same ZIP"),
      ]))
      .expect(1)
      .create_async()
      .await;
    let meta = server
      .mock("GET", "/profile/configs/meta")
      .expect(0)
      .create_async()
      .await;
    assert_eq!(gale_mod_entries().await.unwrap().len(), 3);
    assert_eq!(
      fs::read_to_string(PathBuf::from(bepinex_config_directory()).join("test.cfg")).unwrap(),
      "from same ZIP"
    );
    zip.assert_async().await;
    meta.assert_async().await;
    for key in [
      "GAME_LOCATION",
      GALE_SYNC_CODE_VAR,
      GALE_SYNC_CONFIGS_VAR,
      GALE_SYNC_URL_VAR,
    ] {
      env::remove_var(key);
    }
  }

  #[tokio::test]
  #[serial]
  async fn outage_cache_preserves_validated_sources_and_excludes_legacy_metadata() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    env::set_var("MODS_REPOSITORY", "hexium");
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, server.url());
    let valid = server
      .mock("GET", "/profile/cache")
      .with_body(profile_zip(&manifest()))
      .expect(1)
      .create_async()
      .await;
    let first = fetch_manifest("cache").await.unwrap();
    valid.assert_async().await;
    valid.remove_async().await;
    let outage = server
      .mock("GET", mockito::Matcher::Any)
      .with_status(404)
      .expect(5)
      .create_async()
      .await;
    let cached = fetch_profile("cache").await.unwrap();
    assert!(cached.zip_bytes.is_none());
    assert_eq!(
      manifest_to_mod_entries(&cached.manifest),
      manifest_to_mod_entries(&first)
    );
    assert_eq!(
      cached.manifest.mods.last().unwrap().source.as_deref(),
      Some("Thunderstore")
    );
    fs::write(
      PathBuf::from(mods_staging_directory()).join("gale-legacy.json"),
      serde_json::to_vec(&manifest()).unwrap(),
    )
    .unwrap();
    assert!(
      fetch_manifest("legacy").await.is_err(),
      "old metadata cache cannot be used"
    );
    let unvalidated = CachedSnapshot {
      schema_version: 1,
      manifest: manifest(),
    };
    fs::write(
      cached_manifest_path("invalid"),
      serde_json::to_vec(&unvalidated).unwrap(),
    )
    .unwrap();
    assert!(
      fetch_manifest("invalid").await.is_err(),
      "source-less new cache must also be refused"
    );
    for (code, schema_version, source) in [
      ("wrong-schema", 2, "Thunderstore"),
      ("bad-source", 1, "OtherStore"),
    ] {
      let mut invalid = first.clone();
      invalid.mods[0].source = Some(source.into());
      fs::write(
        cached_manifest_path(code),
        serde_json::to_vec(&CachedSnapshot {
          schema_version,
          manifest: invalid,
        })
        .unwrap(),
      )
      .unwrap();
      assert!(
        fetch_manifest(code).await.is_err(),
        "unvalidated snapshot {code} must be refused"
      );
    }
    outage.assert_async().await;
    for key in ["GAME_LOCATION", "MODS_REPOSITORY", GALE_SYNC_URL_VAR] {
      env::remove_var(key);
    }
  }

  #[tokio::test]
  #[serial]
  async fn invalid_remote_snapshot_does_not_replace_cache_or_configs() {
    let game = tempfile::tempdir().unwrap();
    env::set_var("GAME_LOCATION", game.path());
    env::set_var(GALE_SYNC_CODE_VAR, "invalid");
    env::set_var(GALE_SYNC_CONFIGS_VAR, "true");
    let mut server = Server::new_async().await;
    env::set_var(GALE_SYNC_URL_VAR, server.url());
    let valid = server
      .mock("GET", "/profile/invalid")
      .with_body(profile_zip(&manifest()))
      .create_async()
      .await;
    fetch_manifest("invalid").await.unwrap();
    valid.remove_async().await;
    let cache_before = fs::read(cached_manifest_path("invalid")).unwrap();
    let dest = PathBuf::from(bepinex_config_directory());
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("test.cfg"), "keep").unwrap();
    let bad = server
      .mock("GET", "/profile/invalid")
      .with_body(zip_entries(&[
        ("export.r2x", "community: anothergame\nmods: []"),
        ("BepInEx/config/test.cfg", "replace"),
      ]))
      .expect(1)
      .create_async()
      .await;
    assert!(
      gale_mod_entries().await.is_err(),
      "invalid remote cannot use a stale cache to reconcile"
    );
    assert_eq!(
      fs::read(cached_manifest_path("invalid")).unwrap(),
      cache_before
    );
    assert_eq!(fs::read_to_string(dest.join("test.cfg")).unwrap(), "keep");
    bad.assert_async().await;
    for key in [
      "GAME_LOCATION",
      GALE_SYNC_CODE_VAR,
      GALE_SYNC_CONFIGS_VAR,
      GALE_SYNC_URL_VAR,
    ] {
      env::remove_var(key);
    }
  }
}

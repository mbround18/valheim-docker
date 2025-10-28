use crate::errors::ValheimModError;
use crate::mods::manifest::Manifest;
use crate::utils::normalize_paths::normalize_paths;
use crate::utils::{is_valid_url, parse_mod_string};
use crate::{
  constants::SUPPORTED_FILE_TYPES,
  utils::{common_paths, get_md5_hash, parse_file_name, url_parse_file_type},
};
use fs_extra::dir;
use fs_extra::dir::CopyOptions;
use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, Clone)]
struct ThunderstoreVersionEntry {
  version_number: String,
}

fn thunderstore_list_versions(
  namespace: &str,
  name: &str,
) -> Result<Vec<ThunderstoreVersionEntry>, ValheimModError> {
  use std::time::Duration;

  fn extract_versions(v: &serde_json::Value) -> Option<Vec<ThunderstoreVersionEntry>> {
    // Common shapes:
    // - top-level { versions: [{ version_number: "x.y.z" }] }
    // - nested { package: { versions: [...] } }
    // - direct array [ { version_number: ... } ]
    let parse_arr = |arr: &Vec<serde_json::Value>| -> Vec<ThunderstoreVersionEntry> {
      arr
        .iter()
        .filter_map(|item| item.get("version_number").and_then(|s| s.as_str()))
        .map(|s| ThunderstoreVersionEntry {
          version_number: s.to_string(),
        })
        .collect()
    };

    if let Some(arr) = v.get("versions").and_then(|vv| vv.as_array()) {
      let out = parse_arr(arr);
      if !out.is_empty() {
        return Some(out);
      }
    }
    if let Some(arr) = v
      .get("package")
      .and_then(|p| p.get("versions"))
      .and_then(|vv| vv.as_array())
    {
      let out = parse_arr(arr);
      if !out.is_empty() {
        return Some(out);
      }
    }
    if let Some(arr) = v.as_array() {
      let out = parse_arr(arr);
      if !out.is_empty() {
        return Some(out);
      }
    }
    None
  }

  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(10))
    .user_agent("odin-valheim-docker/1.0 (+https://github.com/mbround18/valheim-docker)")
    .build()
    .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

  let endpoints = vec![
    // Experimental package endpoint (no community in path)
    format!(
      "https://thunderstore.io/api/experimental/package/{}/{}/",
      namespace, name
    ),
    // Community-scoped experimental endpoint (if available)
    format!(
      "https://thunderstore.io/api/experimental/community/valheim/package/{}/{}/",
      namespace, name
    ),
    // Frontend JSON used by website (shape may change but often includes versions)
    format!(
      "https://thunderstore.io/api/experimental/frontend/c/valheim/p/{}/{}/",
      namespace, name
    ),
  ];

  let mut last_err: Option<String> = None;
  for url in endpoints {
    for attempt in 1..=2 {
      log::debug!("Thunderstore version query attempt {}: {}", attempt, url);
      match client.get(&url).send() {
        Ok(resp) => {
          if !resp.status().is_success() {
            last_err = Some(format!("status {} for {}", resp.status(), url));
            continue;
          }
          match resp.json::<serde_json::Value>() {
            Ok(v) => {
              if let Some(out) = extract_versions(&v) {
                if !out.is_empty() {
                  return Ok(out);
                }
                last_err = Some(format!("no versions found in response shape for {}", url));
              } else {
                last_err = Some(format!("unable to parse versions from {}", url));
              }
            }
            Err(e) => {
              last_err = Some(format!("json error for {}: {}", url, e));
            }
          }
        }
        Err(e) => {
          last_err = Some(format!("request error for {}: {}", url, e));
        }
      }
      // brief backoff before next attempt
      std::thread::sleep(Duration::from_millis(500));
    }
  }

  // HTML fallback: scrape latest download link from the package page as a last resort
  let page_url = format!(
    "https://thunderstore.io/c/valheim/p/{}/{}/",
    namespace, name
  );
  match client.get(&page_url).send() {
    Ok(resp) if resp.status().is_success() => match resp.text() {
      Ok(html) => {
        let needle = format!("/package/download/{}/{}/", namespace, name);
        if let Some(pos) = html.find(&needle) {
          // capture characters after needle until next '/'
          let tail = &html[pos + needle.len()..];
          if let Some(end) = tail.find('/') {
            let ver = &tail[..end];
            if !ver.is_empty() {
              return Ok(vec![ThunderstoreVersionEntry {
                version_number: ver.to_string(),
              }]);
            }
          }
        }
        Err(ValheimModError::DownloadError(last_err.unwrap_or_else(
          || "HTML fallback: could not find version".to_string(),
        )))
      }
      Err(e) => Err(ValheimModError::DownloadError(format!(
        "HTML fallback text error: {}",
        e
      ))),
    },
    Ok(resp) => Err(ValheimModError::DownloadError(format!(
      "HTML fallback status {} for {}",
      resp.status(),
      page_url
    ))),
    Err(e) => Err(ValheimModError::DownloadError(format!(
      "HTML fallback request error for {}: {}",
      page_url, e
    ))),
  }
}

fn is_wildcard_version(v: &str) -> bool {
  let lv = v.to_ascii_lowercase();
  lv.contains('*') || lv.contains('x')
}

fn select_version_from_list(
  requested: &str,
  versions: &[ThunderstoreVersionEntry],
) -> Option<String> {
  // Normalize versions list to semver-like where possible; Thunderstore versions may be dot-separated numeric strings.
  // We implement simple matching:
  // - "*" or "x": pick the highest version lexicographically using semver if parseable, else string sort.
  // - "MAJOR.*" or "MAJOR.x": highest version with same major
  // - "MAJOR.MINOR.*" or "MAJOR.MINOR.x": highest with same major/minor
  use semver::Version;

  let req = requested.to_ascii_lowercase();
  let parts: Vec<&str> = req.split('.').collect();

  // Prepare parsed versions with fallback
  let mut parsed: Vec<(Option<Version>, String)> = versions
    .iter()
    .map(|e| {
      let s = e.version_number.clone();
      (Version::parse(&s).ok(), s)
    })
    .collect();

  // Sort descending by semver if available, else by string
  parsed.sort_by(|a, b| match (&a.0, &b.0) {
    (Some(va), Some(vb)) => vb.cmp(va),
    (Some(_), None) => std::cmp::Ordering::Less,
    (None, Some(_)) => std::cmp::Ordering::Greater,
    (None, None) => b.1.cmp(&a.1),
  });

  if req == "*" || req == "x" {
    return parsed.first().map(|(_, s)| s.clone());
  }

  // Helper to check prefix match with wildcards
  let matches_req = |ver_str: &str| {
    let vparts: Vec<&str> = ver_str.split('.').collect();
    if parts.len() == 2 && (parts[1] == "*" || parts[1] == "x") {
      // MAJOR.*
      return vparts.first() == parts.first();
    }
    if parts.len() == 3 && (parts[2] == "*" || parts[2] == "x") {
      // MAJOR.MINOR.*
      return vparts.first() == parts.first() && vparts.get(1) == parts.get(1);
    }
    false
  };

  for (_, s) in &parsed {
    if matches_req(s) {
      return Some(s.clone());
    }
  }
  None
}

pub struct ValheimMod {
  pub(crate) url: String,
  pub(crate) file_type: String,
  /// For download, this is the location of the downloaded ZIP.
  pub(crate) staging_location: PathBuf,
  pub(crate) installed: bool,
  pub(crate) downloaded: bool,
  // Optionally, add fields like author or mod_name if needed later.
}

impl ValheimMod {
  pub fn new(url: &str) -> Self {
    let file_type = url_parse_file_type(url);
    ValheimMod {
      url: url.to_string(),
      file_type,
      staging_location: common_paths::mods_staging_directory().into(),
      installed: false,
      downloaded: false,
    }
  }

  /// Determines whether the mod is a framework by inspecting the extracted files.
  fn is_mod_framework(&self, extract_path: &Path) -> bool {
    debug!("Checking mod if it is a framework like bepinex");
    match Manifest::try_from(extract_path.join("manifest.json")) {
      Ok(manifest) => {
        debug!("Parsed manifest with name: {}", manifest.name);
        manifest.name.to_lowercase().starts_with("bepinex")
      }
      Err(_) => {
        for entry in WalkDir::new(extract_path).into_iter().flatten() {
          if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("winhttp.dll")
          {
            return true;
          }
        }
        false
      }
    }
  }

  /// Compute SHA-256 of a file at the given path.
  fn sha256_hex(path: &Path) -> Result<String, ValheimModError> {
    let mut file = File::open(path).map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
    let mut buf = [0u8; 8192];
    let mut hasher = Sha256::new();
    loop {
      let n = file
        .read(&mut buf)
        .map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
      if n == 0 {
        break;
      }
      hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
  }

  /// Try opening as a ZIP to validate integrity.
  fn is_valid_zip(path: &Path) -> bool {
    matches!(File::open(path).map(ZipArchive::new), Ok(Ok(_)))
  }

  /// Persist a sidecar .sha256 file next to the artifact.
  fn write_sha_sidecar(path: &Path, sha: &str) {
    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
      let mut sidecar = path.to_path_buf();
      sidecar.set_extension(format!(
        "{}sha256",
        path
          .extension()
          .and_then(|e| e.to_str())
          .map(|e| format!("{}.", e))
          .unwrap_or_default()
      ));
      // Fallback simple name if extension building is awkward
      let sidecar = if sidecar
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.ends_with("sha256"))
        .is_some()
      {
        sidecar
      } else {
        let mut p = path.to_path_buf();
        p.set_file_name(format!("{}.sha256", file_name));
        p
      };
      if let Err(e) = std::fs::write(&sidecar, format!("{}  {}\n", sha, file_name)) {
        warn!("Failed to write sha256 sidecar: {}", e);
      }
    }
  }

  /// Download: Downloads the mod ZIP from the URL into the staging location.
  pub fn download(&mut self) -> Result<(), ValheimModError> {
    debug!("Initializing mod download...");
    // For Thunderstore download URLs, validate upfront that the URL isn't 404 to give fast feedback.
    if Self::is_thunderstore_download_url(&self.url) {
      let client = Client::new();
      match client.head(&self.url).send() {
        Ok(resp) => {
          let status = resp.status();
          if status.is_client_error() {
            return Err(ValheimModError::DownloadError(format!(
              "Thunderstore URL not reachable, status: {}",
              status
            )));
          }
        }
        Err(e) => return Err(ValheimModError::DownloadError(e.to_string())),
      }
    }
    // Always derive the staging directory from common paths to avoid stale file paths
    let staging_dir: PathBuf = common_paths::mods_staging_directory().into();
    if !staging_dir.exists() {
      create_dir_all(&staging_dir).unwrap();
    }

    // Pre-compute a likely cache path from the original URL before any network calls.
    let orig_url = Url::parse(&self.url).map_err(|_| ValheimModError::InvalidUrl)?;
    let mut orig_file_type = url_parse_file_type(&self.url);
    if !SUPPORTED_FILE_TYPES.contains(&orig_file_type.as_str()) {
      // Assume zip for mods when type cannot be parsed from URL.
      orig_file_type = "zip".to_string();
    }
    let orig_file_name = parse_file_name(
      &orig_url,
      &format!("{}.{}", get_md5_hash(&self.url), &orig_file_type),
    );
    let orig_cache_path = staging_dir.join(&orig_file_name);

    // Cache hit: URL unchanged and file is a valid ZIP, reuse it.
    if orig_cache_path.exists() && Self::is_valid_zip(&orig_cache_path) {
      debug!("Cache hit for URL; reusing {:?}", orig_cache_path);
      self.staging_location = orig_cache_path;
      self.file_type = orig_file_type;
      self.downloaded = true;
      return Ok(());
    } else if orig_cache_path.exists() {
      warn!(
        "Cached file exists but is not a valid ZIP, removing: {:?}",
        orig_cache_path
      );
      let _ = std::fs::remove_file(&orig_cache_path);
    }

    // Perform request (to resolve redirects and final file type if needed).
    let parsed_url = Url::parse(&self.url).map_err(|_| ValheimModError::InvalidUrl)?;
    let mut response = reqwest::blocking::get(parsed_url)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

    if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
      debug!("Using redirect URL: {}", &self.url);
      self.url = response.url().to_string();
      self.file_type = url_parse_file_type(response.url().as_ref());
      if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
        // Default to zip for mods.
        self.file_type = "zip".to_string();
      }
    }

    let file_name = parse_file_name(
      &Url::parse(&self.url).unwrap(),
      &format!("{}.{}", get_md5_hash(&self.url), &self.file_type),
    );
    let final_path = staging_dir.join(file_name);
    debug!("Downloading to: {:?}", final_path);

    // If the final computed path already exists and is valid, reuse and skip write.
    if final_path.exists() && Self::is_valid_zip(&final_path) {
      debug!("Cache hit after redirect; reusing {:?}", final_path);
      self.staging_location = final_path;
      self.downloaded = true;
      return Ok(());
    } else if final_path.exists() {
      warn!(
        "Existing file at destination is not a valid ZIP, overwriting: {:?}",
        final_path
      );
    }

    let mut file =
      File::create(&final_path).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
    response
      .copy_to(&mut file)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

    // Validate it's a ZIP and compute SHA-256 sidecar.
    if !Self::is_valid_zip(&final_path) {
      error!("Downloaded file is not a valid ZIP: {:?}", final_path);
      return Err(ValheimModError::ZipArchiveError(
        "Invalid ZIP file after download".to_string(),
      ));
    }

    match Self::sha256_hex(&final_path) {
      Ok(sha) => {
        Self::write_sha_sidecar(&final_path, &sha);
        debug!("SHA-256: {}", sha);
      }
      Err(e) => warn!("Failed computing SHA-256: {}", e),
    }

    self.staging_location = final_path;
    self.downloaded = true;
    debug!("Download complete: {}", &self.url);
    debug!("Download output: {:?}", self.staging_location);
    Ok(())
  }

  /// Install: Creates a temporary directory, extracts the ZIP there, validates the mod,
  /// moves extracted files to their final destination, and cleans up the temp directory.
  pub fn install(&mut self) -> Result<(), ValheimModError> {
    // Ensure that the staging location is a file (the downloaded ZIP).
    if self.staging_location.is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory: {:?}",
        self.staging_location
      );
      return Err(ValheimModError::InvalidStagingLocation);
    }

    // Create a temporary directory for extraction.
    let temp_dir = tempdir().map_err(|e| {
      ValheimModError::TempDirCreationError(format!("Failed to create temp dir: {e}"))
    })?;
    debug!("Created temporary directory at {:?}", temp_dir.path());

    // Extract the ZIP file (from staging) into the temporary directory.
    {
      let zip_file = File::open(&self.staging_location)
        .map_err(|e| ValheimModError::FileOpenError(e.to_string()))?;
      let mut archive =
        ZipArchive::new(zip_file).map_err(|e| ValheimModError::ZipArchiveError(e.to_string()))?;
      archive.extract(temp_dir.path()).map_err(|e| {
        error!("Failed to extract archive: {e}");
        ValheimModError::ExtractionError(e.to_string())
      })?;

      normalize_paths(temp_dir.path())
        .map_err(|e| ValheimModError::ExtractionError(e.to_string()))?;
    }
    debug!("Extraction complete to {:?}", temp_dir.path());

    // Validate mod type by inspecting the extracted files.
    let is_framework = self.is_mod_framework(temp_dir.path());

    let mut options = CopyOptions {
      overwrite: true,
      skip_exist: false,
      buffer_size: 0,
      copy_inside: false,
      content_only: true,
      depth: 0,
    };

    let manifest = Manifest::try_from(temp_dir.path().join("manifest.json"))
      .map_err(|e| ValheimModError::ManifestDeserializeError(format!("Ayyre buddy {e}")))?;

    // Move extracted files to the appropriate final destination.
    if is_framework {
      info!("Installing Framework...");
      let final_dir = PathBuf::from(&common_paths::game_directory());
      dir::move_dir(temp_dir.path().join(&manifest.name), &final_dir, &options)
        .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
    } else {
      info!("Installing Mod...");
      let final_dir = PathBuf::from(&common_paths::bepinex_plugin_directory()).join(&manifest.name);
      // If a manifest exists, use its name for a subdirectory.
      create_dir_all(&final_dir)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;

      // Path to the 'plugins' directory within the temp directory
      let plugins_path = temp_dir.path().join("plugins");

      if temp_dir.path().join("Plugins").exists() {
        debug!("Looks like someone used Plugins instead of plugins, lets fix that.");
        dir::move_dir(temp_dir.path().join("Plugins"), &plugins_path, &options)
          .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
      }

      // Check if the 'plugins' directory exists
      if plugins_path.exists() && plugins_path.is_dir() {
        let mut plugin_options = options.clone();
        plugin_options.copy_inside = true;
        dir::move_dir(&plugins_path, &final_dir, &plugin_options)
          .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
        // Set options depth of one to maintain manifest.json
        options.depth = 1
      }
      dir::move_dir(temp_dir, &final_dir, &options)
        .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
    }

    // The temporary directory is automatically removed here.
    self.installed = true;
    Ok(())
  }

  fn is_thunderstore_download_url(url: &str) -> bool {
    url.contains("thunderstore.io/package/download/")
  }
}

impl TryFrom<String> for ValheimMod {
  type Error = ValheimModError;

  fn try_from(url: String) -> Result<Self, Self::Error> {
    if is_valid_url(&url) {
      Ok(ValheimMod::new(&url))
    } else if let Some((author, mod_name, version)) = parse_mod_string(&url) {
      // Resolve wildcards for Thunderstore packages if present
      let v_req = version.to_ascii_lowercase();
      if is_wildcard_version(&v_req) {
        let versions = thunderstore_list_versions(author, mod_name)?;
        if let Some(sel) = select_version_from_list(&v_req, &versions) {
          let constructed_url = format!(
            "https://thunderstore.io/package/download/{}/{}/{}/",
            author, mod_name, sel
          );
          Ok(ValheimMod::new(&constructed_url))
        } else {
          Err(ValheimModError::DownloadError(
            "No matching version found for wildcard".to_string(),
          ))
        }
      } else {
        let constructed_url = format!(
          "https://thunderstore.io/package/download/{}/{}/{}/",
          author, mod_name, version
        );
        Ok(ValheimMod::new(&constructed_url))
      }
    } else {
      Err(ValheimModError::InvalidUrl)
    }
  }
}

#[cfg(test)]
mod install_test {
  use super::*;

  // Helper to create a ValheimMod instance with a given staging location.
  fn valheim_mod_with_staging(url: String, staging: PathBuf) -> ValheimMod {
    ValheimMod {
      url,
      staging_location: staging,
      installed: false,
      downloaded: false,
      file_type: "zip".to_string(),
    }
  }

  #[test]
  fn test_install_framework() {
    // Use a test resource ZIP that represents a framework mod.
    let staging = PathBuf::from("tests/resources/manifest.framework.zip");
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);
  }

  #[test]
  fn test_install_mod() {
    // Use a test resource ZIP that represents a regular mod.
    let staging = PathBuf::from("tests/resources/manifest.mod.zip");
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);
  }
}

#[cfg(test)]
mod thunderstore_tests {
  use super::*;
  use std::env;

  #[test]
  fn transforms_mod_string_to_thunderstore_download_url() {
    let input = "ValheimModding-Jotunn-2.26.0".to_string();
    let vm = ValheimMod::try_from(input).expect("Should construct from mod string");
    assert_eq!(
      vm.url,
      "https://thunderstore.io/package/download/ValheimModding/Jotunn/2.26.0/"
    );
  }

  #[test]
  fn normal_url_is_preserved_in_try_from() {
    let input = "https://example.com/mod.zip".to_string();
    let vm = ValheimMod::try_from(input.clone()).expect("Should construct from URL");
    assert_eq!(vm.url, input);
  }

  #[test]
  fn detects_thunderstore_download_url() {
    let turl = "https://thunderstore.io/package/download/Author/Mod/1.2.3/";
    assert!(ValheimMod::is_thunderstore_download_url(turl));
    let normal = "https://example.com/path/file.zip";
    assert!(!ValheimMod::is_thunderstore_download_url(normal));
  }

  #[test]
  fn select_version_latest_for_full_wildcard() {
    let list = vec![
      ThunderstoreVersionEntry {
        version_number: "1.2.3".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "2.0.0".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "1.9.9".into(),
      },
    ];
    let sel = select_version_from_list("*", &list).unwrap();
    assert_eq!(sel, "2.0.0");
  }

  #[test]
  fn select_version_latest_minor_for_major_wildcard() {
    let list = vec![
      ThunderstoreVersionEntry {
        version_number: "1.2.3".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "1.3.0".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "2.0.0".into(),
      },
    ];
    let sel = select_version_from_list("1.*", &list).unwrap();
    assert_eq!(sel, "1.3.0");
  }

  #[test]
  fn select_version_latest_patch_for_major_minor_wildcard() {
    let list = vec![
      ThunderstoreVersionEntry {
        version_number: "1.2.3".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "1.2.9".into(),
      },
      ThunderstoreVersionEntry {
        version_number: "1.3.0".into(),
      },
    ];
    let sel = select_version_from_list("1.2.*", &list).unwrap();
    assert_eq!(sel, "1.2.9");
  }

  // Optional live test against Thunderstore; requires network and sets an env flag.
  // Enable with: THUNDERSTORE_LIVE_TEST=1 cargo test --package odin thunderstore_live_resolve -- --ignored
  #[test]
  #[ignore]
  fn thunderstore_live_resolve() {
    if env::var("THUNDERSTORE_LIVE_TEST").unwrap_or_default() != "1" {
      eprintln!("skipping live Thunderstore test; set THUNDERSTORE_LIVE_TEST=1 to enable");
      return;
    }

    // Resolve a real wildcard for Jotunn
    let input = "ValheimModding-Jotunn-*".to_string();
    let vm = ValheimMod::try_from(input).expect("Should construct from mod string");
    assert!(
      vm.url
        .starts_with("https://thunderstore.io/package/download/ValheimModding/Jotunn/"),
      "unexpected resolved URL prefix: {}",
      vm.url
    );
    // basic sanity: URL ends with /
    assert!(vm.url.ends_with('/'));
  }
}

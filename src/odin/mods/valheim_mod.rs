use crate::errors::ValheimModError;
use crate::mods::manifest::Manifest;
use crate::utils::http_pool::HttpPool;
use crate::utils::normalize_paths::normalize_paths;
use crate::utils::thunderstore_http::send;
use crate::utils::{
  concurrent_downloads_enabled, is_valid_url, max_concurrent_downloads, parse_mod_string,
  split_repository_prefix, ModRepository,
};
use crate::{
  constants::SUPPORTED_FILE_TYPES,
  utils::{common_paths, get_md5_hash, parse_file_name, url_parse_file_type},
};
use fs_extra::dir;
use fs_extra::dir::CopyOptions;
use log::{debug, error, info, warn};
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, Clone)]
struct ThunderstoreVersionEntry {
  version_number: String,
}

/// Lists the published versions of a package so a wildcard can pick one.
async fn list_versions(
  repo: ModRepository,
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

  let base = repo.base_url();
  let client = HttpPool::global().client();
  let endpoints = match repo {
    ModRepository::Thunderstore => vec![
      // Experimental package endpoint (no community in path)
      format!("{}/api/experimental/package/{}/{}/", base, namespace, name),
      // Community-scoped experimental endpoint (if available)
      format!(
        "{}/api/experimental/community/valheim/package/{}/{}/",
        base, namespace, name
      ),
      // Frontend JSON used by website (shape may change but often includes versions)
      format!(
        "{}/api/experimental/frontend/c/valheim/p/{}/{}/",
        base, namespace, name
      ),
    ],
    // Hexium's package endpoint only carries `latest`; the frontend detail endpoint is the
    // one that lists every version.
    ModRepository::Hexium => vec![format!(
      "{}/api/experimental/frontend/c/valheim/p/{}/{}/",
      base, namespace, name
    )],
  };

  let mut last_err: Option<String> = None;
  for url in endpoints {
    for attempt in 1..=2 {
      log::debug!("{} version query attempt {}: {}", repo, attempt, url);
      match send(
        client.get(&url),
        &url,
        &format!("{} {namespace}/{name}", repo.alias()),
      )
      .await
      {
        Ok(resp) => {
          if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ValheimModError::DownloadError(format!(
              "{repo} rate limit persisted after retries; try again later"
            )));
          }
          if !resp.status().is_success() {
            last_err = Some(format!("status {} for {}", resp.status(), url));
            continue;
          }
          match resp.json::<serde_json::Value>().await {
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
      tokio::time::sleep(Duration::from_millis(500)).await;
    }
  }

  if repo != ModRepository::Thunderstore {
    return Err(ValheimModError::DownloadError(format!(
      "could not list {repo} versions for {namespace}-{name}: {}",
      last_err.unwrap_or_else(|| "no versions returned".to_string())
    )));
  }

  // HTML fallback: scrape latest download link from the package page as a last resort
  let page_url = format!("{}/c/valheim/p/{}/{}/", base, namespace, name);
  match send(
    client.get(&page_url),
    &page_url,
    &format!("thunderstore page {namespace}/{name}"),
  )
  .await
  {
    Ok(resp) if resp.status().is_success() => match resp.text().await {
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

fn thunderstore_download_url(namespace: &str, name: &str, version: &str) -> String {
  format!(
    "{}/package/download/{}/{}/{}/",
    ModRepository::Thunderstore.base_url(),
    namespace,
    name,
    version
  )
}

/// Resolves an exact Hexium version to its download URL.
///
/// Hexium has no Thunderstore-style `/package/download/...` route; the version endpoint's
/// `download_url` (on `cdn.hexium.gg`) is the only way to the file.
async fn hexium_download_url(
  namespace: &str,
  name: &str,
  version: &str,
) -> Result<String, ValheimModError> {
  let base = ModRepository::Hexium.base_url();
  let url = format!("{base}/api/experimental/package/{namespace}/{name}/{version}/");
  let resp = send(
    HttpPool::global().client().get(&url),
    &url,
    &format!("hex {namespace}/{name}/{version}"),
  )
  .await?;

  if resp.status() == reqwest::StatusCode::NOT_FOUND {
    return Err(ValheimModError::DownloadError(format!(
      "{namespace}-{name}-{version} was not found on Hexium ({url})"
    )));
  }
  if !resp.status().is_success() {
    return Err(ValheimModError::DownloadError(format!(
      "status {} for {url}",
      resp.status()
    )));
  }

  let body: serde_json::Value = resp
    .json()
    .await
    .map_err(|e| ValheimModError::DownloadError(format!("json error for {url}: {e}")))?;
  let download_url = body
    .get("download_url")
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .ok_or_else(|| {
      ValheimModError::DownloadError(format!("no download_url in Hexium response for {url}"))
    })?;

  // Absolute in practice; joining also copes with a mirror that returns a relative path.
  Url::parse(&url)
    .and_then(|u| u.join(download_url))
    .map(String::from)
    .map_err(|e| {
      ValheimModError::DownloadError(format!("invalid download_url {download_url:?}: {e}"))
    })
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
  /// `Author-Mod-Version` when resolved from a dependency string. Names the staged file,
  /// because a Hexium CDN URL ends in just the version (`/upload/48/1.8.18.zip`) and two
  /// mods at the same version would otherwise share one cache entry.
  pub(crate) package: Option<String>,
}

impl ValheimMod {
  /// Format bytes as human-readable size (KB, MB, GB)
  fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
      format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
      format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
      format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
      format!("{} B", bytes)
    }
  }

  pub fn new(url: &str) -> Self {
    let file_type = url_parse_file_type(url);
    ValheimMod {
      url: url.to_string(),
      file_type,
      staging_location: common_paths::mods_staging_directory().into(),
      installed: false,
      downloaded: false,
      package: None,
    }
  }

  fn from_package(url: &str, package: String) -> Self {
    ValheimMod {
      package: Some(package),
      ..ValheimMod::new(url)
    }
  }

  /// Staged file name: the package name when known, else the URL's last path segment.
  fn staging_file_name(&self, url: &Url, file_type: &str) -> String {
    match &self.package {
      Some(package) => format!("{package}.{file_type}"),
      None => parse_file_name(url, &format!("{}.{}", get_md5_hash(&self.url), file_type)),
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
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
      use std::fmt::Write as _;
      write!(&mut hex, "{byte:02x}").map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
    }
    Ok(hex)
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

  /// Parallel chunked download for large files.
  ///
  /// Issues up to `MAX_CONCURRENT_DOWNLOADS` concurrent Range requests, writing each chunk
  /// directly to its correct offset so no sorting/buffering pass is needed.
  /// Falls back gracefully: callers only invoke this when the server has already
  /// advertised `Accept-Ranges: bytes` *and* the file exceeds the threshold.
  #[cfg(unix)]
  async fn download_chunked(
    url: &str,
    path: &Path,
    total_size: u64,
  ) -> Result<(), ValheimModError> {
    use std::os::unix::fs::FileExt;

    const CHUNK_SIZE: u64 = 4 * 1024 * 1024; // 4 MB per chunk
    let max_workers = max_concurrent_downloads();

    let num_chunks = total_size.div_ceil(CHUNK_SIZE);
    info!(
      "🚀 Parallel download: {} in {} chunks ({} concurrent)",
      Self::format_bytes(total_size),
      num_chunks,
      max_workers
    );

    // Pre-allocate the file so workers can write to arbitrary offsets safely.
    {
      let file = File::create(path).map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
      file
        .set_len(total_size)
        .map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_workers));
    let url = Arc::new(url.to_string());
    let client = HttpPool::global().client();
    // Shared writable handle — write_at uses pwrite64 so concurrent non-overlapping
    // writes are safe without any additional locking.
    let file = Arc::new(
      std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| ValheimModError::FileOpenError(e.to_string()))?,
    );

    let mut join_set: tokio::task::JoinSet<Result<(), ValheimModError>> =
      tokio::task::JoinSet::new();

    for i in 0..num_chunks {
      let start_byte = i * CHUNK_SIZE;
      let end_byte = std::cmp::min(start_byte + CHUNK_SIZE - 1, total_size - 1);
      let url = url.clone();
      let sem = semaphore.clone();
      let file = file.clone();

      join_set.spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        let resp = send(
          client
            .get(url.as_str())
            .header("Range", format!("bytes={}-{}", start_byte, end_byte)),
          &url,
          &format!("chunk {i}"),
        )
        .await
        .map_err(|e| ValheimModError::DownloadError(format!("chunk {i}: {e}")))?;

        // 206 Partial Content is expected; 200 is tolerated (full body served).
        if resp.status() != reqwest::StatusCode::PARTIAL_CONTENT && !resp.status().is_success() {
          return Err(ValheimModError::DownloadError(format!(
            "chunk {i} status {}",
            resp.status()
          )));
        }

        let data = resp
          .bytes()
          .await
          .map_err(|e| ValheimModError::DownloadError(format!("chunk {i} body: {e}")))?;

        tokio::task::spawn_blocking(move || {
          file
            .write_at(&data, start_byte)
            .map(|_| ())
            .map_err(|e| ValheimModError::FileCreateError(format!("chunk {i} write: {e}")))
        })
        .await
        .map_err(|e| ValheimModError::DownloadError(format!("chunk {i} task: {e}")))?
      });
    }

    while let Some(result) = join_set.join_next().await {
      match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
          join_set.abort_all();
          return Err(e);
        }
        Err(e) => {
          join_set.abort_all();
          return Err(ValheimModError::DownloadError(format!("chunk task: {e}")));
        }
      }
    }

    info!("✓ Parallel download complete ({num_chunks} chunks assembled)");
    Ok(())
  }

  /// Download: Downloads the mod ZIP from the URL into the staging location.
  pub async fn download(&mut self) -> Result<(), ValheimModError> {
    debug!("Initializing mod download...");
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
    let orig_file_name = self.staging_file_name(&orig_url, &orig_file_type);
    let orig_cache_path = staging_dir.join(&orig_file_name);

    // Cache hit: URL unchanged. For ZIPs require valid ZIP; for non-zip types (dll, cfg) accept cached file.
    if orig_cache_path.exists() {
      if orig_file_type == "zip" {
        if Self::is_valid_zip(&orig_cache_path) {
          if let Ok(metadata) = std::fs::metadata(&orig_cache_path) {
            let size = Self::format_bytes(metadata.len());
            info!("⚡ Cache hit: reusing {} from cache", size);
            debug!("   Path: {:?}", orig_cache_path);
          } else {
            info!("⚡ Cache hit: reusing cached file");
          }
          self.staging_location = orig_cache_path;
          self.file_type = orig_file_type;
          self.downloaded = true;
          return Ok(());
        } else {
          warn!(
            "Cached file exists but is not a valid ZIP, removing: {:?}",
            orig_cache_path
          );
          let _ = std::fs::remove_file(&orig_cache_path);
        }
      } else {
        if let Ok(metadata) = std::fs::metadata(&orig_cache_path) {
          let size = Self::format_bytes(metadata.len());
          info!("⚡ Cache hit: reusing {} (non-zip)", size);
          debug!("   Path: {:?}", orig_cache_path);
        } else {
          info!("⚡ Cache hit: reusing cached file (non-zip)");
        }
        self.staging_location = orig_cache_path;
        self.file_type = orig_file_type;
        self.downloaded = true;
        return Ok(());
      }
    }

    // Perform request (to resolve redirects and final file type if needed).
    let parsed_url = Url::parse(&self.url).map_err(|_| ValheimModError::InvalidUrl)?;
    debug!("⬇️  Downloading from: {}", self.url);
    let response = send(
      HttpPool::global().client().get(parsed_url),
      &self.url,
      &format!("download {}", self.url),
    )
    .await?;

    if !response.status().is_success() {
      return Err(ValheimModError::DownloadError(format!(
        "status {} for {}",
        response.status(),
        self.url
      )));
    }

    if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
      debug!("Using redirect URL: {}", self.url);
      self.url = response.url().to_string();
      self.file_type = url_parse_file_type(response.url().as_ref());
      if !SUPPORTED_FILE_TYPES.contains(&self.file_type.as_str()) {
        // Default to zip for mods.
        self.file_type = "zip".to_string();
      }
    }

    let file_name = self.staging_file_name(&Url::parse(&self.url).unwrap(), &self.file_type);
    let final_path = staging_dir.join(file_name);
    debug!("Downloading to: {:?}", final_path);

    // If the final computed path already exists, reuse it for non-zip types or validate ZIPs.
    if final_path.exists() {
      if self.file_type == "zip" {
        if Self::is_valid_zip(&final_path) {
          if let Ok(metadata) = std::fs::metadata(&final_path) {
            let size = Self::format_bytes(metadata.len());
            info!("⚡ Cache hit (post-redirect): reusing {}", size);
            debug!("   Path: {:?}", final_path);
          } else {
            info!("⚡ Cache hit (post-redirect): reusing cached file");
          }
          self.staging_location = final_path;
          self.downloaded = true;
          return Ok(());
        } else {
          warn!(
            "Existing file at destination is not a valid ZIP, overwriting: {:?}",
            final_path
          );
        }
      } else {
        if let Ok(metadata) = std::fs::metadata(&final_path) {
          let size = Self::format_bytes(metadata.len());
          info!("⚡ Cache hit (post-redirect): reusing {} (non-zip)", size);
          debug!("   Path: {:?}", final_path);
        } else {
          info!("⚡ Cache hit (post-redirect): reusing cached file (non-zip)");
        }
        self.staging_location = final_path;
        self.downloaded = true;
        return Ok(());
      }
    }

    // Check whether the server supports parallel Range downloads.
    // We inspect headers from the already-established GET response so no extra round-trip is needed.
    const PARALLEL_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MB
    let parallel_size: Option<u64> = {
      let accepts_ranges = response
        .headers()
        .get("accept-ranges")
        .and_then(|v| v.to_str().ok())
        .map(|v| v != "none")
        .unwrap_or(false);
      let content_length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
      if accepts_ranges {
        content_length.filter(|&l| l >= PARALLEL_THRESHOLD)
      } else {
        None
      }
    };

    let start_time = std::time::Instant::now();

    #[cfg(unix)]
    if let Some(total_size) = parallel_size.filter(|_| concurrent_downloads_enabled()) {
      // Drop the initial response — we don't need its body; chunk tasks open their own connections.
      drop(response);
      Self::download_chunked(&self.url, &final_path, total_size).await?;
      let elapsed = start_time.elapsed();
      let size = Self::format_bytes(
        std::fs::metadata(&final_path)
          .map(|m| m.len())
          .unwrap_or(total_size),
      );
      info!("✓ Downloaded {} in {:.1}s", size, elapsed.as_secs_f64());
    } else {
      info!("📦 Downloading mod...");
      let bytes = response
        .bytes()
        .await
        .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
      let elapsed = start_time.elapsed();
      let size = Self::format_bytes(bytes.len() as u64);
      std::fs::write(&final_path, &bytes)
        .map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
      info!("✓ Downloaded {} in {:.1}s", size, elapsed.as_secs_f64());
    }

    #[cfg(not(unix))]
    {
      let _ = parallel_size; // unused on non-Unix; fall through to standard download
      info!("📦 Downloading mod...");
      let bytes = response
        .bytes()
        .await
        .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
      let elapsed = start_time.elapsed();
      let size = Self::format_bytes(bytes.len() as u64);
      std::fs::write(&final_path, &bytes)
        .map_err(|e| ValheimModError::FileCreateError(e.to_string()))?;
      info!("✓ Downloaded {} in {:.1}s", size, elapsed.as_secs_f64());
    }

    // Validate based on file type. ZIP must be valid; non-zip types (dll, cfg) are accepted.
    if self.file_type == "zip" {
      if !Self::is_valid_zip(&final_path) {
        error!("Downloaded file is not a valid ZIP: {:?}", final_path);
        return Err(ValheimModError::ZipArchiveError(
          "Invalid ZIP file after download".to_string(),
        ));
      }
    } else {
      debug!(
        "Downloaded non-zip file ({}), skipping ZIP validation: {:?}",
        self.file_type, final_path
      );
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
    debug!("Download complete: {}", self.url);
    debug!("Download output: {:?}", self.staging_location);
    Ok(())
  }

  /// Install: Creates a temporary directory, extracts the ZIP there, validates the mod,
  /// moves extracted files to their final destination, and cleans up the temp directory.
  pub fn install(&mut self) -> Result<(), ValheimModError> {
    self.install_with_report().map(|_| ())
  }

  /// Like `install()`, but returns the destination paths that were installed.
  ///
  /// This is primarily used by `odin mod:install --from-var` so it can persist
  /// cleanup metadata (installed paths + staged artifact) across restarts.
  pub fn install_with_report(&mut self) -> Result<Vec<PathBuf>, ValheimModError> {
    // Ensure that the staging location is a file (the downloaded ZIP or single-file mod).
    if self.staging_location.is_dir() {
      error!(
        "Failed to install mod! Staging location is a directory: {:?}",
        self.staging_location
      );
      return Err(ValheimModError::InvalidStagingLocation);
    }

    // Special-case: if this is a single-file DLL plugin, copy it directly into the plugins directory.
    if self.file_type.eq_ignore_ascii_case("dll") {
      info!("Installing DLL plugin directly into plugins directory...");
      let plugin_dir = PathBuf::from(&common_paths::bepinex_plugin_directory());
      create_dir_all(&plugin_dir)
        .map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))?;
      let file_name = self
        .staging_location
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(ValheimModError::InvalidStagingLocation)?;
      let dest = plugin_dir.join(file_name);
      std::fs::copy(&self.staging_location, &dest)
        .map_err(|e| ValheimModError::FileMoveError(e.to_string()))?;
      self.installed = true;
      return Ok(vec![dest]);
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
      let installed_root = final_dir.join(&manifest.name);
      self.installed = true;
      Ok(vec![installed_root])
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

      self.installed = true;
      Ok(vec![final_dir])
    }
  }

  /// Async constructor for a `MODS` entry: a URL, or a dependency string optionally
  /// prefixed with a repository alias (`ts:`, `hex:`). Unprefixed dependency strings use
  /// `MODS_REPOSITORY`. Wildcard versions are resolved against the chosen repository.
  pub async fn async_from_url(input: &str) -> Result<Self, ValheimModError> {
    // Strip the prefix first: `hex:Author-Mod-1.0.0` would otherwise parse as a URL
    // with a `hex` scheme.
    let (prefix, entry) = split_repository_prefix(input);
    if is_valid_url(entry) {
      return Ok(ValheimMod::new(entry));
    }
    let (author, mod_name, version) = parse_mod_string(entry).ok_or(ValheimModError::InvalidUrl)?;
    let repo = prefix.unwrap_or_else(ModRepository::from_env);

    let version = if is_wildcard_version(version) {
      let versions = list_versions(repo, author, mod_name).await?;
      select_version_from_list(&version.to_ascii_lowercase(), &versions).ok_or_else(|| {
        ValheimModError::DownloadError(format!(
          "No matching version found for wildcard {entry} on {repo}"
        ))
      })?
    } else {
      version.to_string()
    };

    let url = match repo {
      ModRepository::Thunderstore => thunderstore_download_url(author, mod_name, &version),
      ModRepository::Hexium => hexium_download_url(author, mod_name, &version).await?,
    };
    Ok(ValheimMod::from_package(
      &url,
      format!("{author}-{mod_name}-{version}"),
    ))
  }
}

impl TryFrom<String> for ValheimMod {
  type Error = ValheimModError;

  fn try_from(url: String) -> Result<Self, Self::Error> {
    let (prefix, entry) = split_repository_prefix(&url);
    if is_valid_url(entry) {
      return Ok(ValheimMod::new(entry));
    }
    let (author, mod_name, version) = parse_mod_string(entry).ok_or(ValheimModError::InvalidUrl)?;

    // For TryFrom (synchronous), only exact Thunderstore versions can be built without a
    // lookup. Wildcards and Hexium must use async_from_url instead.
    if is_wildcard_version(version) {
      return Err(ValheimModError::DownloadError(
        "Wildcard versions require async resolution. Use ValheimMod::async_from_url().".to_string(),
      ));
    }
    let repo = prefix.unwrap_or_else(ModRepository::from_env);
    if repo != ModRepository::Thunderstore {
      return Err(ValheimModError::DownloadError(format!(
        "{repo} mod strings require async resolution. Use ValheimMod::async_from_url()."
      )));
    }
    Ok(ValheimMod::from_package(
      &thunderstore_download_url(author, mod_name, version),
      format!("{author}-{mod_name}-{version}"),
    ))
  }
}

#[cfg(test)]
mod install_test {
  use super::*;
  use serial_test::serial;

  // Helper to create a ValheimMod instance with a given staging location.
  fn valheim_mod_with_staging(url: String, staging: PathBuf) -> ValheimMod {
    ValheimMod {
      url,
      staging_location: staging,
      installed: false,
      downloaded: false,
      file_type: "zip".to_string(),
      package: None,
    }
  }

  #[tokio::test]
  #[serial]
  async fn test_install_framework() {
    // Use a test resource ZIP that represents a framework mod.
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    let game_dir_str = game_dir.to_string_lossy().to_string();
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir_str);

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let staging = PathBuf::from(format!(
      "{}/tests/resources/manifest.framework.zip",
      manifest_dir
    ));
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);

    drop(tmp);
  }

  #[tokio::test]
  #[serial]
  async fn test_install_mod() {
    // Use a test resource ZIP that represents a regular mod.
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    let game_dir_str = game_dir.to_string_lossy().to_string();
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir_str);

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let staging = PathBuf::from(format!("{}/tests/resources/manifest.mod.zip", manifest_dir));
    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/test.zip".to_string(), staging);
    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);

    drop(tmp);
  }

  #[tokio::test]
  #[serial]
  async fn test_install_dll() {
    // Create a temporary game directory and staging DLL
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();

    let game_dir_str = game_dir.to_string_lossy().to_string();
    // Point GAME_LOCATION to our temp game dir
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir_str);

    let staging = tmp.path().join("dummy.dll");
    std::fs::write(&staging, b"dummy dll content").unwrap();

    let mut mod_inst =
      valheim_mod_with_staging("https://example.com/dummy.dll".to_string(), staging);
    mod_inst.file_type = "dll".to_string();

    let result = mod_inst.install();
    assert!(result.is_ok(), "{:?}", result.err());
    assert!(mod_inst.installed);

    let dest = PathBuf::from(common_paths::bepinex_plugin_directory()).join("dummy.dll");
    assert!(dest.exists(), "DLL should exist at {:?}", dest);

    // Verify the copy actually happened
    let content = std::fs::read(&dest).expect("should read dll");
    assert_eq!(content, b"dummy dll content", "DLL content should match");

    // Don't drop tmp until after assertions
    drop(tmp);
  }
}

#[cfg(test)]
mod thunderstore_tests {
  use super::*;
  use mockito::Server;
  use serial_test::serial;
  use std::env;

  #[tokio::test]
  #[serial]
  async fn transforms_mod_string_to_thunderstore_download_url() {
    let input = "ValheimModding-Jotunn-2.26.0".to_string();
    let vm = ValheimMod::try_from(input).expect("Should construct from mod string");
    assert_eq!(
      vm.url,
      "https://thunderstore.io/package/download/ValheimModding/Jotunn/2.26.0/"
    );
  }

  #[tokio::test]
  async fn normal_url_is_preserved_in_try_from() {
    let input = "https://example.com/mod.zip".to_string();
    let vm = ValheimMod::try_from(input.clone()).expect("Should construct from URL");
    assert_eq!(vm.url, input);
  }

  #[tokio::test]
  async fn select_version_latest_for_full_wildcard() {
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

  #[tokio::test]
  async fn select_version_latest_minor_for_major_wildcard() {
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

  #[tokio::test]
  async fn select_version_latest_patch_for_major_minor_wildcard() {
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
  #[tokio::test]
  #[ignore]
  async fn thunderstore_live_resolve() {
    if env::var("THUNDERSTORE_LIVE_TEST").unwrap_or_default() != "1" {
      eprintln!("skipping live Thunderstore test; set THUNDERSTORE_LIVE_TEST=1 to enable");
      return;
    }

    // Resolve a real wildcard for Jotunn. This must go through async_from_url:
    // TryFrom deliberately rejects wildcards because it cannot await the lookup.
    for pattern in ["ValheimModding-Jotunn-*", "ValheimModding-Jotunn-2.*"] {
      let vm = ValheimMod::async_from_url(pattern)
        .await
        .unwrap_or_else(|e| panic!("{pattern} should resolve against the live API: {e}"));
      let prefix = "https://thunderstore.io/package/download/ValheimModding/Jotunn/";
      assert!(
        vm.url.starts_with(prefix),
        "unexpected resolved URL prefix for {pattern}: {}",
        vm.url
      );
      assert!(
        vm.url.ends_with('/'),
        "resolved URL should end with a slash"
      );

      // The wildcard must have been replaced by a concrete version.
      let version = vm.url[prefix.len()..].trim_end_matches('/');
      assert!(
        !version.is_empty() && version.chars().next().is_some_and(|c| c.is_ascii_digit()),
        "expected a concrete version for {pattern}, got {version:?}"
      );
      if pattern.contains("2.") {
        assert!(
          version.starts_with("2."),
          "MAJOR wildcard must stay within its major, got {version}"
        );
      }
      eprintln!("{pattern} -> {version}");
    }
  }

  // Optional live test to download a real DLL from GitHub releases; requires network and sets an env flag.
  // Enable with: VALHEIMPLUS_LIVE_TEST=1 cargo test -p odin download_dll_live -- --ignored
  #[tokio::test]
  #[ignore]
  async fn download_dll_live() {
    if env::var("VALHEIMPLUS_LIVE_TEST").unwrap_or_default() != "1" {
      eprintln!("skipping live DLL download test; set VALHEIMPLUS_LIVE_TEST=1 to enable");
      return;
    }

    // Use a temporary game directory for isolated staging
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let url =
      "https://github.com/Grantapher/ValheimPlus/releases/download/0.9.16.2/ValheimPlus.dll"
        .to_string();
    let mut vm = ValheimMod::new(&url);

    // First download should fetch the file
    let r = vm.download().await;
    assert!(r.is_ok(), "{:?}", r.err());
    assert!(vm.downloaded);
    assert_eq!(vm.file_type, "dll");
    assert!(vm.staging_location.exists());

    // Verify the file looks like a DLL by extension and non-zero size
    assert_eq!(
      vm.staging_location
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or(""),
      "dll"
    );
    let md = std::fs::metadata(&vm.staging_location).expect("metadata");
    assert!(md.len() > 0, "downloaded file should not be empty");

    // Second download should hit the cache and reuse the same staging file
    let mut vm2 = ValheimMod::new(&url);
    let r2 = vm2.download().await;
    assert!(r2.is_ok(), "{:?}", r2.err());
    assert!(vm2.downloaded);
    assert_eq!(vm2.staging_location, vm.staging_location);
  }

  // Synthetic tests (run in CI) using mockito to provide deterministic download endpoints
  #[tokio::test]
  #[serial]
  async fn synthetic_download_dll_and_install() {
    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/ValheimPlus.dll")
      .with_status(200)
      .with_header("content-type", "application/octet-stream")
      .with_body("DUMMYDLL")
      .create();

    let url = format!("{}/ValheimPlus.dll", server.url());

    // Isolate game location
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let mut vm = ValheimMod::new(&url);
    let r = vm.download().await;
    assert!(r.is_ok(), "{:?}", r.err());
    assert!(vm.downloaded);
    assert_eq!(vm.file_type, "dll");
    assert!(vm.staging_location.exists());

    // Install should copy DLL into plugins
    let r2 = vm.install();
    assert!(r2.is_ok(), "{:?}", r2.err());
    let dest_plugin = PathBuf::from(common_paths::bepinex_plugin_directory()).join(
      vm.staging_location
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap(),
    );
    assert!(dest_plugin.exists());
  }

  #[tokio::test]
  #[serial]
  async fn synthetic_download_zip_and_install() {
    use std::io::Cursor;
    use std::io::Write;

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

    let mut server = Server::new_async().await;
    let _m = server
      .mock("GET", "/testmod.zip")
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(buf)
      .create();

    let url = format!("{}/testmod.zip", server.url());
    let mut vm = ValheimMod::new(&url);
    let r = vm.download().await;
    assert!(r.is_ok(), "{:?}", r.err());
    assert!(vm.downloaded);
    assert_eq!(vm.file_type, "zip");
    assert!(vm.staging_location.exists());

    // Install to isolated game dir
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    std::env::set_var(crate::constants::GAME_LOCATION, &game_dir);

    let r2 = vm.install();
    assert!(r2.is_ok(), "{:?}", r2.err());
    let dest = PathBuf::from(common_paths::bepinex_plugin_directory())
      .join("testmod")
      .join("myplugin.dll");
    assert!(dest.exists());
  }
}

#[cfg(test)]
mod wildcard_resolution_tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  const BASE_URL_VAR: &str = "THUNDERSTORE_BASE_URL";

  fn versions_json(versions: &[&str]) -> String {
    let entries: Vec<String> = versions
      .iter()
      .map(|v| format!("{{\"version_number\":\"{v}\"}}"))
      .collect();
    format!("{{\"versions\":[{}]}}", entries.join(","))
  }

  const PKG_PATH: &str = "/api/experimental/package/Author/Mod/";

  #[tokio::test]
  #[serial]
  async fn full_wildcard_resolves_to_latest_version() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
      .mock("GET", PKG_PATH)
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(versions_json(&["1.2.3", "2.0.1", "1.9.9"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-*").await.unwrap();
    assert_eq!(
      vmod.url,
      format!("{}/package/download/Author/Mod/2.0.1/", server.url())
    );
    remove_var(BASE_URL_VAR);
  }

  #[tokio::test]
  #[serial]
  async fn major_wildcard_stays_within_major() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
      .mock("GET", PKG_PATH)
      .with_status(200)
      .with_body(versions_json(&["1.2.3", "2.0.1", "1.9.9"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-1.*").await.unwrap();
    assert!(
      vmod.url.ends_with("/Author/Mod/1.9.9/"),
      "expected 1.9.9, got {}",
      vmod.url
    );
    remove_var(BASE_URL_VAR);
  }

  #[tokio::test]
  #[serial]
  async fn minor_wildcard_stays_within_minor() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
      .mock("GET", PKG_PATH)
      .with_status(200)
      .with_body(versions_json(&["1.2.3", "1.2.10", "1.3.0"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-1.2.*")
      .await
      .unwrap();
    assert!(
      vmod.url.ends_with("/Author/Mod/1.2.10/"),
      "expected 1.2.10, got {}",
      vmod.url
    );
    remove_var(BASE_URL_VAR);
  }

  #[tokio::test]
  #[serial]
  async fn exact_version_makes_no_network_call() {
    let mut server = mockito::Server::new_async().await;
    let never = server.mock("GET", PKG_PATH).expect(0).create_async().await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-1.2.3")
      .await
      .unwrap();
    assert!(vmod.url.ends_with("/Author/Mod/1.2.3/"));
    never.assert_async().await;
    remove_var(BASE_URL_VAR);
  }

  /// The rate-limit backoff must not break wildcards: a 429 on the first try should
  /// be retried, not treated as "package not found".
  #[tokio::test]
  #[serial]
  async fn wildcard_survives_a_rate_limited_first_attempt() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
      .mock("GET", PKG_PATH)
      .with_status(429)
      .with_header("retry-after", "0")
      .expect(1)
      .create_async()
      .await;
    let ok = server
      .mock("GET", PKG_PATH)
      .with_status(200)
      .with_body(versions_json(&["3.1.0", "3.0.0"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-*").await.unwrap();
    assert!(
      vmod.url.ends_with("/Author/Mod/3.1.0/"),
      "expected 3.1.0, got {}",
      vmod.url
    );
    limited.assert_async().await;
    ok.assert_async().await;
    remove_var(BASE_URL_VAR);
  }

  /// A 404 on the primary endpoint is not retryable and must fall through to the
  /// community-scoped endpoint rather than aborting resolution.
  #[tokio::test]
  #[serial]
  async fn wildcard_falls_through_to_secondary_endpoint() {
    let mut server = mockito::Server::new_async().await;
    let _primary = server
      .mock("GET", PKG_PATH)
      .with_status(404)
      .expect_at_least(1)
      .create_async()
      .await;
    let secondary = server
      .mock(
        "GET",
        "/api/experimental/community/valheim/package/Author/Mod/",
      )
      .with_status(200)
      .with_body(versions_json(&["4.2.0"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-*").await.unwrap();
    assert!(
      vmod.url.ends_with("/Author/Mod/4.2.0/"),
      "expected 4.2.0, got {}",
      vmod.url
    );
    secondary.assert_async().await;
    remove_var(BASE_URL_VAR);
  }

  /// Last-resort HTML scrape of the package page still works when every API shape fails.
  #[tokio::test]
  #[serial]
  async fn wildcard_falls_back_to_html_scrape() {
    let mut server = mockito::Server::new_async().await;
    for path in [
      PKG_PATH,
      "/api/experimental/community/valheim/package/Author/Mod/",
      "/api/experimental/frontend/c/valheim/p/Author/Mod/",
    ] {
      server
        .mock("GET", path)
        .with_status(404)
        .expect_at_least(1)
        .create_async()
        .await;
    }
    let page = server
      .mock("GET", "/c/valheim/p/Author/Mod/")
      .with_status(200)
      .with_body("<html><a href=\"/package/download/Author/Mod/5.5.5/\">Download</a></html>")
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("Author-Mod-*").await.unwrap();
    assert!(
      vmod.url.ends_with("/Author/Mod/5.5.5/"),
      "expected 5.5.5, got {}",
      vmod.url
    );
    page.assert_async().await;
    remove_var(BASE_URL_VAR);
  }

  #[tokio::test]
  #[serial]
  async fn wildcard_with_no_matching_major_errors() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
      .mock("GET", PKG_PATH)
      .with_status(200)
      .with_body(versions_json(&["1.0.0", "2.0.0"]))
      .create_async()
      .await;
    set_var(BASE_URL_VAR, server.url());

    match ValheimMod::async_from_url("Author-Mod-9.*").await {
      Ok(m) => panic!("expected no match for 9.*, resolved to {}", m.url),
      Err(e) => assert!(
        e.to_string().contains("No matching version"),
        "unexpected error: {e}"
      ),
    }
    remove_var(BASE_URL_VAR);
  }
}

#[cfg(test)]
mod repository_resolution_tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};
  use std::io::{Cursor, Write};

  const HEXIUM_BASE_URL_VAR: &str = "HEXIUM_BASE_URL";
  const THUNDERSTORE_BASE_URL_VAR: &str = "THUNDERSTORE_BASE_URL";
  const MODS_REPOSITORY_VAR: &str = "MODS_REPOSITORY";

  fn clear_env() {
    for var in [
      HEXIUM_BASE_URL_VAR,
      THUNDERSTORE_BASE_URL_VAR,
      MODS_REPOSITORY_VAR,
    ] {
      remove_var(var);
    }
  }

  /// Mocks Hexium's version endpoint for `package` (`Author/Mod/1.0.0`).
  async fn mock_hexium_version(
    server: &mut mockito::ServerGuard,
    package: &str,
    download_url: &str,
  ) -> mockito::Mock {
    server
      .mock(
        "GET",
        format!("/api/experimental/package/{package}/").as_str(),
      )
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(serde_json::json!({ "download_url": download_url }).to_string())
      .create_async()
      .await
  }

  fn zip_with_manifest(name: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    {
      let mut zipw = zip::ZipWriter::new(Cursor::new(&mut buf));
      let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
      zipw.start_file("manifest.json", options).unwrap();
      zipw
        .write_all(serde_json::json!({ "name": name }).to_string().as_bytes())
        .unwrap();
      zipw
        .start_file(format!("plugins/{name}.dll"), options)
        .unwrap();
      zipw.write_all(name.as_bytes()).unwrap();
      zipw.finish().unwrap();
    }
    buf
  }

  #[tokio::test]
  #[serial]
  async fn hex_prefix_resolves_the_cdn_download_url() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let cdn = format!("{}/upload/48/1.8.18.zip", server.url());
    let version = mock_hexium_version(&mut server, "Azumatt/AzuCraftyBoxes/1.8.18", &cdn).await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("hex:Azumatt-AzuCraftyBoxes-1.8.18")
      .await
      .unwrap();
    assert_eq!(vmod.url, cdn);
    assert_eq!(
      vmod.package.as_deref(),
      Some("Azumatt-AzuCraftyBoxes-1.8.18")
    );
    version.assert_async().await;
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn mods_repository_routes_unprefixed_strings_to_hexium() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let cdn = format!("{}/upload/1/1.0.0.zip", server.url());
    let version = mock_hexium_version(&mut server, "Author/Mod/1.0.0", &cdn).await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());
    set_var(MODS_REPOSITORY_VAR, "hexium");

    let vmod = ValheimMod::async_from_url("Author-Mod-1.0.0")
      .await
      .unwrap();
    assert_eq!(vmod.url, cdn);
    version.assert_async().await;
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn ts_prefix_overrides_a_hexium_default() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let never = server
      .mock("GET", mockito::Matcher::Any)
      .expect(0)
      .create_async()
      .await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());
    set_var(MODS_REPOSITORY_VAR, "hex");

    let vmod = ValheimMod::async_from_url("ts:Author-Mod-1.0.0")
      .await
      .unwrap();
    assert_eq!(
      vmod.url,
      "https://thunderstore.io/package/download/Author/Mod/1.0.0/"
    );
    never.assert_async().await;
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn hexium_wildcard_lists_versions_from_the_frontend_endpoint() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let listing = server
      .mock("GET", "/api/experimental/frontend/c/valheim/p/Author/Mod/")
      .with_status(200)
      .with_body(
        serde_json::json!({ "versions": [
          { "version_number": "2.0.0" },
          { "version_number": "1.10.0" },
          { "version_number": "1.2.3" },
        ]})
        .to_string(),
      )
      .create_async()
      .await;
    let cdn = format!("{}/upload/9/1.10.0.zip", server.url());
    let version = mock_hexium_version(&mut server, "Author/Mod/1.10.0", &cdn).await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("hex:Author-Mod-1.*")
      .await
      .unwrap();
    assert_eq!(vmod.url, cdn);
    assert_eq!(vmod.package.as_deref(), Some("Author-Mod-1.10.0"));
    listing.assert_async().await;
    version.assert_async().await;
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn missing_hexium_version_names_the_repository() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let _missing = server
      .mock("GET", "/api/experimental/package/Author/Mod/9.9.9/")
      .with_status(404)
      .with_body(r#"{"detail":"Not found."}"#)
      .create_async()
      .await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let err = match ValheimMod::async_from_url("hex:Author-Mod-9.9.9").await {
      Ok(m) => panic!("expected a failure, resolved to {}", m.url),
      Err(e) => e.to_string(),
    };
    assert!(
      err.contains("Author-Mod-9.9.9 was not found on Hexium"),
      "unexpected error: {err}"
    );
    clear_env();
  }

  /// Hexium has no HTML page scrape to fall back on, so a missing listing must fail with
  /// a message that says which repository was asked.
  #[tokio::test]
  #[serial]
  async fn missing_hexium_listing_names_the_repository() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let _missing = server
      .mock("GET", "/api/experimental/frontend/c/valheim/p/Author/Mod/")
      .with_status(404)
      .expect_at_least(1)
      .create_async()
      .await;
    let no_scrape = server
      .mock("GET", "/c/valheim/p/Author/Mod/")
      .expect(0)
      .create_async()
      .await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let err = match ValheimMod::async_from_url("hex:Author-Mod-*").await {
      Ok(m) => panic!("expected a failure, resolved to {}", m.url),
      Err(e) => e.to_string(),
    };
    assert!(
      err.contains("could not list Hexium versions for Author-Mod"),
      "unexpected error: {err}"
    );
    no_scrape.assert_async().await;
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn relative_hexium_download_url_is_joined_to_the_base() {
    clear_env();
    let mut server = mockito::Server::new_async().await;
    let _version = mock_hexium_version(
      &mut server,
      "Author/Mod/1.0.0",
      "/files/Author-Mod-1.0.0.zip",
    )
    .await;
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let vmod = ValheimMod::async_from_url("hex:Author-Mod-1.0.0")
      .await
      .unwrap();
    assert_eq!(
      vmod.url,
      format!("{}/files/Author-Mod-1.0.0.zip", server.url())
    );
    clear_env();
  }

  #[tokio::test]
  #[serial]
  async fn urls_ignore_the_repository_setting() {
    clear_env();
    set_var(MODS_REPOSITORY_VAR, "hexium");
    for input in [
      "https://example.com/mod.zip",
      "hex:https://example.com/mod.zip",
    ] {
      let vmod = ValheimMod::async_from_url(input).await.unwrap();
      assert_eq!(vmod.url, "https://example.com/mod.zip", "{input}");
      assert!(vmod.package.is_none(), "{input}");
    }
    clear_env();
  }

  #[test]
  #[serial]
  fn try_from_defers_hexium_to_async_resolution() {
    clear_env();
    let err = ValheimMod::try_from("hex:Author-Mod-1.0.0".to_string())
      .err()
      .expect("Hexium needs a lookup, so TryFrom must refuse it")
      .to_string();
    assert!(err.contains("async"), "unexpected error: {err}");

    let vmod = ValheimMod::try_from("ts:Author-Mod-1.0.0".to_string()).unwrap();
    assert_eq!(
      vmod.url,
      "https://thunderstore.io/package/download/Author/Mod/1.0.0/"
    );
    clear_env();
  }

  /// Hexium CDN URLs end in just the version (`/upload/<id>/1.0.0.zip`), so two mods at
  /// the same version must still stage to different files. Before the package name was
  /// used for staging, the second mod was a cache hit on the first mod's zip.
  #[tokio::test]
  #[serial]
  async fn hexium_mods_sharing_a_version_stage_separately() {
    clear_env();
    let tmp = tempfile::tempdir().expect("tempdir");
    let game_dir = tmp.path().join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    set_var(crate::constants::GAME_LOCATION, &game_dir);

    let mut server = mockito::Server::new_async().await;
    let mut mocks = Vec::new();
    for (id, name) in [(1, "First"), (2, "Second")] {
      let cdn_path = format!("/upload/{id}/1.0.0.zip");
      let cdn_url = format!("{}{cdn_path}", server.url());
      mocks.push(mock_hexium_version(&mut server, &format!("Author/{name}/1.0.0"), &cdn_url).await);
      mocks.push(
        server
          .mock("GET", cdn_path.as_str())
          .with_status(200)
          .with_header("content-type", "application/zip")
          .with_body(zip_with_manifest(name))
          .create_async()
          .await,
      );
    }
    set_var(HEXIUM_BASE_URL_VAR, server.url());

    let mut staged = Vec::new();
    for name in ["First", "Second"] {
      let mut vmod = ValheimMod::async_from_url(&format!("hex:Author-{name}-1.0.0"))
        .await
        .unwrap();
      vmod.download().await.unwrap();
      vmod.install().unwrap();
      staged.push(vmod.staging_location.clone());
    }

    assert_ne!(staged[0], staged[1], "both mods staged to {:?}", staged[0]);
    assert!(
      staged[0].ends_with("Author-First-1.0.0.zip"),
      "{:?}",
      staged[0]
    );
    let plugins = PathBuf::from(common_paths::bepinex_plugin_directory());
    assert!(plugins.join("First").join("First.dll").exists());
    assert!(plugins.join("Second").join("Second.dll").exists());
    for mock in &mocks {
      mock.assert_async().await;
    }
    clear_env();
  }

  // Optional live test against Hexium; requires network.
  // Enable with: HEXIUM_LIVE_TEST=1 cargo test -p odin hexium_live_resolve -- --ignored
  #[tokio::test]
  #[ignore]
  #[serial]
  async fn hexium_live_resolve() {
    if std::env::var("HEXIUM_LIVE_TEST").unwrap_or_default() != "1" {
      eprintln!("skipping live Hexium test; set HEXIUM_LIVE_TEST=1 to enable");
      return;
    }
    clear_env();

    for (pattern, expected_prefix) in [
      ("hex:ValheimModding-Jotunn-*", "ValheimModding-Jotunn-"),
      ("hex:ValheimModding-Jotunn-2.*", "ValheimModding-Jotunn-2."),
      (
        "hex:Azumatt-AzuCraftyBoxes-1.8.18",
        "Azumatt-AzuCraftyBoxes-1.8.18",
      ),
    ] {
      let vmod = ValheimMod::async_from_url(pattern)
        .await
        .unwrap_or_else(|e| panic!("{pattern} should resolve against the live API: {e}"));
      assert!(
        vmod.url.starts_with("https://cdn.hexium.gg/"),
        "{pattern} should resolve to the Hexium CDN, got {}",
        vmod.url
      );
      let package = vmod.package.as_deref().expect("package name");
      assert!(
        package.starts_with(expected_prefix),
        "{pattern} resolved to {package}"
      );
      eprintln!("{pattern} -> {package} ({})", vmod.url);
    }
  }
}

use log::{debug, error, info, warn};

use std::{
  env, fs, io,
  path::{Path, PathBuf},
  process::ExitStatus,
  time::{SystemTime, UNIX_EPOCH},
};

use crate::utils::common_paths;
use crate::utils::fs::remove_path_cautious;
use crate::utils::steamcmd_args::{compose_app_update_arg, BetaConfig};
use crate::{constants, steamcmd::run_with_retries, utils::get_working_dir};
use crate::{executable::parse_command_args, utils::environment};
use walkdir::WalkDir;

pub fn is_installed() -> bool {
  Path::new(&get_working_dir())
    .join(constants::VALHEIM_EXECUTABLE_NAME)
    .exists()
}

pub fn add_beta_args(
  args: &mut Vec<String>,
  use_public_beta: bool,
  beta_branch: String,
  beta_password: String,
) {
  let cfg = BetaConfig::from_parts(use_public_beta, beta_branch, beta_password);
  if cfg.beta_in_effect() {
    info!("Using {} beta branch", cfg.branch);
  }
  // Only append beta flags if in effect; validate flag uses env toggle
  if cfg.beta_in_effect() {
    args.push(format!("-beta {}", cfg.branch));
    if cfg.use_public_beta && !cfg.is_backwards_compatible_branch() {
      args.push(format!("-betapassword {}", cfg.password));
    }
  }
  if environment::is_env_var_truthy_with_default("VALIDATE_ON_INSTALL", true) {
    args.push(String::from("validate"));
  } else {
    debug!("Skipping SteamCMD validate: VALIDATE_ON_INSTALL=0");
  }
}

fn add_additional_args(args: &mut Vec<String>) {
  if let Ok(extra_args) = env::var("ADDITIONAL_STEAMCMD_ARGS") {
    let additional_args = String::from(extra_args.trim_start_matches('"').trim_end_matches('"'));
    if !additional_args.is_empty() {
      debug!("Adding additional arguments! {additional_args}");
      args.push(additional_args)
    }
  }
}

pub fn install(app_id: i64) -> io::Result<ExitStatus> {
  let staged_install = staged_updates_enabled();
  let live_install_dir = get_working_dir();
  let install_dir = resolve_install_dir();

  // Preflight: show space/inode summary before attempting install/update
  log_space_overview();

  // Optionally clear Steam caches to avoid corrupted state between runs
  clear_steam_cache_if_enabled();

  // Preflight: ensure we can write to critical directories (fail fast if any are not writable)
  if let Err(e) = preflight_write_checks(staged_install) {
    error!("Preflight write check failed: {e}");
    return Err(io::Error::new(io::ErrorKind::PermissionDenied, e));
  }

  if staged_install {
    if let Err(e) = prepare_staging_dir(Path::new(&install_dir)) {
      error!("Failed to prepare staged install dir: {}", e);
      return Err(io::Error::other(e));
    }
  }

  // Stash BepInEx folder only if a clean install will run, then restore after
  let maybe_stash = if environment::fetch_var("CLEAN_INSTALL", "0") == "1" && !staged_install {
    stash_bepinex_before_clean()
  } else {
    None
  };

  // Optional: clean install directory (delete contents of workdir) before running steamcmd.
  // Skip live clean when staged updates are enabled.
  if staged_install && environment::fetch_var("CLEAN_INSTALL", "0") == "1" {
    warn!(
      "CLEAN_INSTALL=1 ignored because STAGED_UPDATES=1 (live dir is not cleaned pre-promotion)"
    );
  } else {
    clean_install_if_enabled();
  }

  // Pre-install logging: current build and whether beta branch will be used
  let prev_build = crate::server::try_get_current_build_id();
  if let Some(build) = &prev_build {
    info!("Current build: {build}");
  } else {
    info!("Current build: unknown (manifest not found)");
  }

  let beta_cfg = BetaConfig::from_env();
  let beta_in_effect = beta_cfg.beta_in_effect();
  if beta_in_effect {
    info!("Installing using beta branch: {}", beta_cfg.branch);
  } else {
    info!("Installing using default/stable branch");
  }

  info!("Installing {} to {}", app_id, install_dir);
  let login = "+login anonymous".to_string();
  let force_install_dir = format!("+force_install_dir {}", install_dir);
  // Build SteamCMD args with order:
  // 1) +@ control vars  2) +force_install_dir  3) +login  4) optional verbose  5) +app_update {id} [beta flags] [validate]
  let mut args = vec![
    // +@ controls first
    String::from("+@NoPromptForPassword"),
    String::from("1"),
    String::from("+@ShutdownOnFailedCommand"),
    String::from("1"),
    String::from("+@sSteamCmdForcePlatformType"),
    String::from("linux"),
    String::from("+@sSteamCmdForcePlatformBitness"),
    String::from("64"),
    // then force install dir and login
    force_install_dir,
    login,
  ];

  if environment::is_env_var_truthy_with_default("DEBUG_MODE", false) {
    args.push(String::from("verbose"));
  }

  // Compose +app_update with beta flags and optional validate as trailing args
  let app_update = compose_app_update_arg(
    app_id,
    &beta_cfg,
    environment::is_env_var_truthy_with_default("VALIDATE_ON_INSTALL", true),
  );
  args.push(app_update);

  // Append any additional raw args, if provided
  // Append any additional raw args, if provided (handled consistently with helper)
  add_additional_args(&mut args);

  // remove the call to steamccmd from args to avoid duplication
  args.retain(|arg| arg != "/usr/bin/steamcmd");

  // for args if it has a space, reduce and append it so its a seperate arg
  args = flatten_args(args);
  args = parse_command_args(args);

  let mut result = run_with_retries(&args);

  if staged_install {
    match &result {
      Ok(status) if status.success() => {
        let staged_dir = Path::new(&install_dir);
        let live_dir = Path::new(&live_install_dir);
        if let Err(e) = validate_staged_install(staged_dir, app_id) {
          error!("Staged install validation failed: {}", e);
          result = Err(io::Error::other(e));
        } else if let Err(e) = promote_staged_install(staged_dir, live_dir, app_id) {
          error!("Failed to promote staged install: {}", e);
          result = Err(io::Error::other(e));
        } else {
          info!(
            "Promoted staged install from {} to {}",
            staged_dir.display(),
            live_dir.display()
          );
        }
      }
      _ => {}
    }
  }

  // Attempt restore of stashed BepInEx regardless of result (best effort)
  if let Err(e) = restore_bepinex_after_install(maybe_stash) {
    error!("Failed to restore BepInEx after install: {}", e);
  }

  // If the install failed, collect diagnostics to help identify issues (e.g., low disk space)
  if let Err(ref e) = result {
    error!("steamcmd execution error: {e}");
    collect_install_diagnostics(None);
  } else if let Ok(ref status) = result {
    if !status.success() {
      collect_install_diagnostics(status.code());
    }
  }

  // Post-install logging: new build id
  let post_build = crate::server::try_get_current_build_id();
  match (prev_build.as_deref(), post_build.as_deref()) {
    (Some(prev), Some(post)) if prev == post => {
      info!("No change in build version: {post}");
    }
    (Some(prev), Some(post)) => {
      if beta_in_effect {
        info!(
          "Installed update from build {prev} -> {post} (beta: {})",
          beta_cfg.branch
        );
      } else {
        info!("Installed update from build {prev} -> {post} (stable)");
      }
    }
    (_, Some(post)) => {
      if beta_in_effect {
        info!("Installed to build {post} (beta: {})", beta_cfg.branch);
      } else {
        info!("Installed to build {post} (stable)");
      }
    }
    _ => info!("Install complete; build id not found."),
  }

  result
}

fn staged_updates_enabled() -> bool {
  environment::fetch_var("STAGED_UPDATES", "0") == "1"
}

fn resolve_install_dir() -> String {
  if staged_updates_enabled() {
    environment::fetch_var("STAGED_INSTALL_DIR", "/home/steam/.staging/valheim-pending")
  } else {
    get_working_dir()
  }
}

fn validate_staged_install(staged_dir: &Path, app_id: i64) -> Result<(), String> {
  let exe = staged_dir.join(constants::VALHEIM_EXECUTABLE_NAME);
  if !exe.exists() {
    return Err(format!(
      "staged executable missing after install: {}",
      exe.display()
    ));
  }
  let manifest = staged_dir
    .join("steamapps")
    .join(format!("appmanifest_{}.acf", app_id));
  if !manifest.exists() {
    return Err(format!(
      "staged appmanifest missing after install: {}",
      manifest.display()
    ));
  }
  Ok(())
}

fn promote_staged_install(staged_dir: &Path, live_dir: &Path, app_id: i64) -> Result<(), String> {
  validate_staged_install(staged_dir, app_id)?;
  fs::create_dir_all(live_dir).map_err(|e| {
    format!(
      "failed to ensure live install dir {}: {}",
      live_dir.display(),
      e
    )
  })?;

  let rollback_root = rollback_root_dir();
  fs::create_dir_all(&rollback_root).map_err(|e| {
    format!(
      "failed to create rollback dir {}: {}",
      rollback_root.display(),
      e
    )
  })?;

  let mut created_paths: Vec<PathBuf> = Vec::new();
  let mut backed_up_paths: Vec<(PathBuf, PathBuf)> = Vec::new();

  for entry in WalkDir::new(staged_dir).into_iter().filter_map(Result::ok) {
    let src_path = entry.path();
    let rel = src_path
      .strip_prefix(staged_dir)
      .map_err(|e| format!("failed to compute relative staged path: {}", e))?;
    if rel.as_os_str().is_empty() {
      continue;
    }
    let dst_path = live_dir.join(rel);
    let ft = entry.file_type();

    if ft.is_dir() {
      if !dst_path.exists() {
        fs::create_dir_all(&dst_path).map_err(|e| {
          rollback_promotion(&created_paths, &backed_up_paths);
          format!("failed to create dir {}: {}", dst_path.display(), e)
        })?;
        created_paths.push(dst_path.clone());
      }
      continue;
    }

    if let Some(parent) = dst_path.parent() {
      fs::create_dir_all(parent).map_err(|e| {
        rollback_promotion(&created_paths, &backed_up_paths);
        format!("failed to create parent dir {}: {}", parent.display(), e)
      })?;
    }

    if dst_path.exists() {
      if dst_path.is_file() {
        let backup_path = rollback_root.join(rel);
        if let Some(parent) = backup_path.parent() {
          fs::create_dir_all(parent).map_err(|e| {
            rollback_promotion(&created_paths, &backed_up_paths);
            format!("failed to create rollback dir {}: {}", parent.display(), e)
          })?;
        }
        fs::copy(&dst_path, &backup_path).map_err(|e| {
          rollback_promotion(&created_paths, &backed_up_paths);
          format!(
            "failed to backup existing file {}: {}",
            dst_path.display(),
            e
          )
        })?;
        backed_up_paths.push((dst_path.clone(), backup_path));
      } else {
        rollback_promotion(&created_paths, &backed_up_paths);
        return Err(format!(
          "cannot replace non-file path during promotion: {}",
          dst_path.display()
        ));
      }
    } else {
      created_paths.push(dst_path.clone());
    }

    fs::copy(src_path, &dst_path).map_err(|e| {
      rollback_promotion(&created_paths, &backed_up_paths);
      format!(
        "failed to copy staged file {} -> {}: {}",
        src_path.display(),
        dst_path.display(),
        e
      )
    })?;
  }

  Ok(())
}

fn rollback_promotion(created_paths: &[PathBuf], backed_up_paths: &[(PathBuf, PathBuf)]) {
  for created in created_paths.iter().rev() {
    if created.exists() {
      let _ = remove_path_cautious(created);
    }
  }
  for (dst, backup) in backed_up_paths.iter().rev() {
    if backup.exists() {
      let _ = fs::copy(backup, dst);
    }
  }
}

fn rollback_root_dir() -> PathBuf {
  let ts = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
  PathBuf::from("/home/steam/.staging")
    .join("rollback")
    .join(ts.to_string())
}

fn stash_bepinex_before_clean() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
  use std::fs;
  use std::path::PathBuf;
  let be_dir = common_paths::bepinex_directory();
  let be_path = PathBuf::from(&be_dir);
  if !be_path.exists() {
    debug!("BepInEx not present, skipping stash");
    return None;
  }
  let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/steam"));
  let stage_root = PathBuf::from(home).join(".stage");
  let stage_be = stage_root.join("BepInEx");
  if let Err(e) = fs::create_dir_all(&stage_root) {
    debug!("Failed to create stage dir {}: {}", stage_root.display(), e);
    return None;
  }
  // Remove any previous stage copy to ensure fresh state
  if stage_be.exists() {
    let _ = fs::remove_dir_all(&stage_be);
  }
  debug!(
    "Stashing BepInEx {} -> {}",
    be_path.display(),
    stage_be.display()
  );
  match fs_extra::dir::copy(&be_path, &stage_root, &fs_extra::dir::CopyOptions::new()) {
    Ok(_) => Some((be_path, stage_be)),
    Err(e) => {
      debug!("Failed to stash BepInEx: {}", e);
      None
    }
  }
}

fn restore_bepinex_after_install(
  stash: Option<(std::path::PathBuf, std::path::PathBuf)>,
) -> Result<(), String> {
  use std::fs;
  if stash.is_none() {
    return Ok(());
  }
  let (be_dest, stage_be) = stash.unwrap();
  if !stage_be.exists() {
    return Ok(());
  }
  debug!(
    "Restoring BepInEx {} -> {}",
    stage_be.display(),
    be_dest.display()
  );
  // Ensure destination BepInEx dir exists
  if let Some(parent) = be_dest.parent() {
    let _ = fs::create_dir_all(parent);
  }
  let _ = fs::create_dir_all(&be_dest);
  // Copy staged BepInEx back into destination directory; overwrite existing files
  let mut opts = fs_extra::dir::CopyOptions::new();
  opts.overwrite = true;
  opts.copy_inside = true;
  fs_extra::dir::copy(&stage_be, &be_dest, &opts).map_err(|e| e.to_string())?;
  Ok(())
}

fn clean_install_if_enabled() {
  let should_clean = environment::fetch_var("CLEAN_INSTALL", "0") == "1";
  if !should_clean {
    debug!("Skipping clean install: CLEAN_INSTALL=0");
    return;
  }
  let server_dir_str = common_paths::game_directory();
  let dir = Path::new(&server_dir_str);
  if !dir.exists() {
    debug!(
      "Clean install: server dir does not exist yet: {}",
      dir.display()
    );
    return;
  }
  warn!(
    "Clean install: deleting contents of server dir {}",
    dir.display()
  );
  if let Err(e) = remove_dir_contents(dir) {
    error!(
      "Clean install failed to remove contents of server dir {}: {}",
      dir.display(),
      e
    );
  } else {
    info!("Clean install: server dir cleaned: {}", dir.display());
  }

  // Additionally clear Steam appcache to avoid stale state on clean installs
  let appcache = Path::new("/home/steam/Steam").join("appcache");
  if appcache.exists() {
    match std::fs::remove_dir_all(&appcache) {
      Ok(_) => info!(
        "Clean install: cleared Steam appcache: {}",
        appcache.display()
      ),
      Err(e) => warn!(
        "Clean install: failed to clear Steam appcache {}: {}",
        appcache.display(),
        e
      ),
    }
  } else {
    debug!(
      "Clean install: Steam appcache not present: {}",
      appcache.display()
    );
  }
}

fn remove_dir_contents(dir: &Path) -> std::io::Result<()> {
  use std::fs;
  if !dir.is_dir() {
    return Ok(());
  }
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    // Do not attempt to remove the directory itself; only contents
    if let Err(e) = remove_path_cautious(&path) {
      debug!("Failed to remove {}: {}", path.display(), e);
      return Err(e);
    }
  }
  Ok(())
}

fn clear_steam_cache_if_enabled() {
  let should_clear = environment::fetch_var("CLEAR_STEAM_CACHE_ON_INSTALL", "1") == "1";
  if !should_clear {
    debug!("Skipping Steam cache clear: CLEAR_STEAM_CACHE_ON_INSTALL=0");
    return;
  }
  match clear_steam_cache() {
    Ok(removed) => {
      if removed.is_empty() {
        debug!("Steam cache: nothing to clear");
      } else {
        debug!("Steam cache: cleared {} item(s)", removed.len());
        for p in removed {
          debug!("removed: {}", p);
        }
      }
    }
    Err(e) => {
      // Not fatal; log and continue
      error!("Steam cache clear encountered errors: {}", e);
    }
  }
}

fn clear_steam_cache() -> Result<Vec<String>, String> {
  let mut removed: Vec<String> = Vec::new();
  let mut errs: Vec<String> = Vec::new();

  let roots = vec![
    "/home/steam/Steam",
    "/home/steam/.steam",
    "/home/steam/.local/share/Steam",
    "/home/steam/steamcmd",
  ];
  let subdirs = vec![
    "appcache",
    "depotcache",
    "logs",
    "package",
    "steamapps/downloading",
    "steamapps/temp",
    "steamapps/shadercache",
  ];

  for root in &roots {
    for sub in &subdirs {
      let path = Path::new(root).join(sub);
      if path.exists() {
        match remove_path_cautious(&path) {
          Ok(_) => removed.push(path.display().to_string()),
          Err(e) => errs.push(format!("{}: {}", path.display(), e)),
        }
      }
    }
  }

  // Also clear a limited set of steam temp files in /tmp (safe patterns).
  let tmp = Path::new("/tmp");
  if let Ok(read_dir) = std::fs::read_dir(tmp) {
    for entry in read_dir.flatten() {
      let name = entry.file_name();
      let name = name.to_string_lossy();
      if name.starts_with("steam") || name.starts_with("Steam") || name.starts_with(".steam") {
        let p = entry.path();
        match remove_path_cautious(&p) {
          Ok(_) => removed.push(p.display().to_string()),
          Err(e) => errs.push(format!("{}: {}", p.display(), e)),
        }
      }
    }
  }

  if errs.is_empty() {
    Ok(removed)
  } else {
    Err(errs.join("; "))
  }
}

fn preflight_write_checks(staged_install: bool) -> Result<(), String> {
  use std::fs::{remove_file, OpenOptions};
  use std::io::Write;
  use std::time::{SystemTime, UNIX_EPOCH};

  let workdir = get_working_dir();
  let paths = vec![
    "/home/steam/Steam".to_string(),
    "/home/steam/.steam".to_string(),
    "/home/steam/.local/share/Steam".to_string(),
    "/home/steam/steamcmd".to_string(),
    workdir,
    "/home/steam/backups".to_string(),
    "/tmp".to_string(),
  ];
  let mut paths = paths;
  if staged_install {
    paths.push(environment::fetch_var(
      "STAGED_INSTALL_DIR",
      "/home/steam/.staging/valheim-pending",
    ));
    paths.push("/home/steam/.staging".to_string());
  }

  let now_ns = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_nanos();

  for p in paths {
    debug!("Testing write access: {}", p);
    let dir = Path::new(&p);
    if !dir.exists() {
      debug!("Skipping write check (missing): {}", p);
      continue;
    }
    if !dir.is_dir() {
      return Err(format!("path is not a directory: {}", p));
    }
    let probe = dir.join(format!(".write_test_{}", now_ns));
    match OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(false)
      .open(&probe)
    {
      Ok(mut file) => {
        if let Err(e) = file.write_all(b"ok") {
          let _ = remove_file(&probe);
          return Err(format!("WRITE FAIL {}: {}", p, e));
        }
      }
      Err(e) => {
        return Err(format!("WRITE FAIL {}: {}", p, e));
      }
    }
    if let Err(e) = remove_file(&probe) {
      // Not fatal for writability, but report cleanup issue
      debug!("cleanup failed for {}: {}", probe.display(), e);
    }
    debug!("Write check ok: {}", p);
  }

  Ok(())
}

fn prepare_staging_dir(dir: &Path) -> Result<(), String> {
  if dir.exists() {
    remove_dir_contents(dir).map_err(|e| {
      format!(
        "failed to clear existing staged dir contents {}: {}",
        dir.display(),
        e
      )
    })?;
  }
  fs::create_dir_all(dir)
    .map_err(|e| format!("failed to create staged dir {}: {}", dir.display(), e))?;
  Ok(())
}

/// Split any argument entries containing whitespace into multiple args.
/// This helps when upstream code accidentally pushes combined tokens.
pub(crate) fn flatten_args(args: Vec<String>) -> Vec<String> {
  args
    .iter()
    .flat_map(|arg| {
      arg
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<String>>()
    })
    .collect()
}

/// When an install attempt fails, gather basic system diagnostics to aid debugging
fn collect_install_diagnostics(exit_code: Option<i32>) {
  use shared::system::fs_usage_for;
  let workdir = get_working_dir();
  if let Some(code) = exit_code {
    error!("steamcmd exited with code: {code}");
  } else {
    error!("steamcmd returned an error without an exit code");
  }

  let gb = 1024.0 * 1024.0 * 1024.0;
  info!("Collecting concise filesystem diagnostics...");
  for p in [&workdir, "/home/steam/backups", "/home/steam/.steam", "/"] {
    if let Some(fs) = fs_usage_for(p) {
      info!(
        "Path {}: used={:.2}GiB ({:.1}%), avail={:.2}GiB, total={:.2}GiB | inodes used={}/{} ({:.1}%)",
        fs.path,
        fs.used_bytes as f64 / gb,
        fs.used_percent,
        fs.available_bytes as f64 / gb,
        fs.total_bytes as f64 / gb,
        fs.inodes_used,
        fs.inodes_total,
        fs.inodes_used_percent,
      );
    }
    if let Some(perm) = shared::system::perm_summary_for(p) {
      info!("{}", shared::system::format_perm_summary(&perm));
    }
  }
  info!("Diagnostics complete. Free space if low and retry the install.");
}

/// Emit a brief disk/inode space overview before install/update begins
fn log_space_overview() {
  use shared::system::{collect_system_metrics, fs_usage_for};
  let workdir = get_working_dir();
  info!("Preflight: system and filesystem summary...");
  let sys = collect_system_metrics();
  let gb = 1024.0 * 1024.0 * 1024.0;
  debug!(
    "System: mem_used={:.2}GiB / {:.2}GiB, swap_used={:.2}GiB / {:.2}GiB, cpus={}, loadavg={{{:.2}, {:.2}, {:.2}}}",
    sys.used_memory_bytes as f64 / gb,
    sys.total_memory_bytes as f64 / gb,
    sys.used_swap_bytes as f64 / gb,
    sys.total_swap_bytes as f64 / gb,
    sys.cpu_num_logical,
    sys.load_average_one,
    sys.load_average_five,
    sys.load_average_fifteen,
  );

  for p in [&workdir, "/home/steam/backups", "/home/steam/.steam", "/"] {
    if let Some(fs) = fs_usage_for(p) {
      let line = format!(
        "Path {}: used={:.2}GiB ({:.1}%), avail={:.2}GiB, total={:.2}GiB | inodes used={}/{} ({:.1}%)",
        fs.path,
        fs.used_bytes as f64 / gb,
        fs.used_percent,
        fs.available_bytes as f64 / gb,
        fs.total_bytes as f64 / gb,
        fs.inodes_used,
        fs.inodes_total,
        fs.inodes_used_percent,
      );
      let low_space = fs.used_percent >= 85.0 || fs.available_bytes < (5.0 * gb) as u64;
      let high_inodes = fs.inodes_used_percent >= 85.0;
      if low_space || high_inodes {
        warn!("{}", line);
      } else {
        debug!("{}", line);
      }
    }
    if let Some(perm) = shared::system::perm_summary_for(p) {
      let msg = shared::system::format_perm_summary(&perm);
      if !perm.can_write {
        warn!("{}", msg);
      } else {
        debug!("{}", msg);
      }
    }
  }
}

// remove_path_cautious moved to utils::fs for reuse

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;
  use test_case::test_case;

  #[test_case(
    false,
    "default_beta_branch".to_string(),
    "default_beta_password".to_string(),
    vec!["validate"]
  )]
  #[test_case(
    true,
    "public-test".to_string(),
    "yesimadebackups".to_string(),
    vec!["-beta public-test", "-betapassword yesimadebackups", "validate"]
  )]
  #[test_case(
    false,
    "default_preml".to_string(),
    "default_beta_password".to_string(),
    vec!["-beta default_preml", "validate"]
  )]
  #[test_case(
    true,
    "default_old".to_string(),
    "default_beta_password".to_string(),
    vec!["-beta default_old","validate"]
  )]
  fn test_no_beta(
    use_public_beta: bool,
    beta_branch: String,
    beta_password: String,
    expected: Vec<&str>,
  ) {
    let mut args = vec![];
    add_beta_args(&mut args, use_public_beta, beta_branch, beta_password);
    assert_eq!(args, expected);
  }

  #[test]
  fn test_additional_steamcmd_args_only() {
    let mut args = vec!["example".to_string()];
    env::set_var("ADDITIONAL_STEAMCMD_ARGS", "+@nCSClientRateLimitKbps 0");
    add_additional_args(&mut args);
    assert_eq!(args.join(" "), "example +@nCSClientRateLimitKbps 0");
    env::remove_var("ADDITIONAL_STEAMCMD_ARGS");
  }

  #[test]
  fn test_disable_validate_via_env() {
    let mut args = vec![];
    // public beta off, but that's irrelevant; main target is to skip validate
    env::set_var("VALIDATE_ON_INSTALL", "0");
    add_beta_args(
      &mut args,
      false,
      "default_beta_branch".to_string(),
      "default_beta_password".to_string(),
    );
    env::remove_var("VALIDATE_ON_INSTALL");
    assert!(args.iter().all(|a| a != "validate"));
  }

  #[test]
  fn test_flatten_args_splits_simple_pair() {
    let input = vec!["+login anonymous".to_string(), "+quit".to_string()];
    let out = flatten_args(input);
    assert_eq!(out, vec!["+login", "anonymous", "+quit"]);
  }

  #[test]
  fn test_flatten_args_handles_multiple_spaces() {
    let input = vec!["  +force_install_dir   /srv/valheim  ".to_string()];
    let out = flatten_args(input);
    assert_eq!(out, vec!["+force_install_dir", "/srv/valheim"]);
  }

  #[test]
  fn test_compose_app_update_with_beta_and_validate() {
    let cfg = BetaConfig::from_parts(true, "public-test".to_string(), "pass".to_string());
    let s = compose_app_update_arg(896660, &cfg, true);
    assert_eq!(
      s,
      "+app_update 896660 -beta public-test -betapassword pass validate"
    );
  }

  #[test]
  fn test_compose_app_update_no_beta_no_validate() {
    let cfg = BetaConfig::from_parts(false, "ignored".to_string(), "".to_string());
    let s = compose_app_update_arg(896660, &cfg, false);
    assert_eq!(s, "+app_update 896660");
  }
}

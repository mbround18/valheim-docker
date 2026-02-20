use log::{debug, error, info};

use regex::Regex;
use std::{env, fs, io::ErrorKind, path::Path, process::exit};

use crate::{
  constants, files::config::load_config, server, steamcmd::output_with_retries,
  utils::get_working_dir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInfo {
  current_build_id: String,
  latest_build_id: String,
}

impl UpdateInfo {
  pub fn new() -> Self {
    let current_build_id = get_current_build_id();
    let latest_build_id = get_latest_build_id();
    Self::internal_new(current_build_id, latest_build_id)
  }

  #[cfg(test)]
  pub fn new_testing(manifest_contents: &str, app_info_output: &str) -> Self {
    debug!("Manifest contents:\n{manifest_contents}");
    debug!("App info output:\n{app_info_output}");
    let current_build_id = extract_build_id_from_manifest(manifest_contents).to_string();
    let latest_build_id = extract_build_id_from_app_info(app_info_output).to_string();

    Self::internal_new(current_build_id, latest_build_id)
  }

  fn internal_new(current_build_id: String, latest_build_id: String) -> Self {
    Self {
      current_build_id,
      latest_build_id,
    }
  }

  pub fn update_available(&self) -> bool {
    self.current_build_id != self.latest_build_id
  }
}

impl Default for UpdateInfo {
  fn default() -> Self {
    Self::new()
  }
}

pub fn update_is_available() -> bool {
  let info = UpdateInfo::new();
  debug!("{info:#?}");

  info.update_available()
}

pub fn update_server() {
  // Shutdown the server if it's running
  let server_was_running = server::is_running();
  if server_was_running {
    server::blocking_shutdown();
  }

  // Detect current build (if available) and whether beta branch will be used
  let prev_build = crate::server::try_get_current_build_id();
  if let Some(build) = &prev_build {
    info!("Current build: {build}");
  } else {
    info!("Current build: unknown (manifest not found)");
  }

  let beta_branch = env::var("BETA_BRANCH").unwrap_or_else(|_| "public-test".to_string());
  let use_public_beta = crate::utils::environment::fetch_var("USE_PUBLIC_BETA", "0").eq("1");
  let is_backwards_compatible_branch =
    ["default_preal", "default_old", "default_preml"].contains(&beta_branch.as_str());
  let beta_in_effect = is_backwards_compatible_branch || use_public_beta;
  if beta_in_effect {
    info!("Updating using beta branch: {beta_branch}");
  } else {
    info!("Updating using default/stable branch");
  }

  // Update the installation
  if let Err(e) = server::install(constants::GAME_ID) {
    error!("Failed to install server: {e}");
    exit(1);
  }

  // Read the build after update
  let post_build = crate::server::try_get_current_build_id();
  match (prev_build.as_deref(), post_build.as_deref()) {
    (Some(prev), Some(post)) if prev == post => {
      info!("No change in build version: {post}");
    }
    (Some(prev), Some(post)) => {
      if beta_in_effect {
        info!("Updated from build {prev} -> {post} (beta: {beta_branch})");
      } else {
        info!("Updated from build {prev} -> {post} (stable)");
      }
    }
    (_, Some(post)) => {
      if beta_in_effect {
        info!("Updated to build {post} (beta: {beta_branch})");
      } else {
        info!("Updated to build {post} (stable)");
      }
    }
    _ => info!("Update complete; build id not found."),
  }

  // Bring the server up if it was running before
  if server_was_running {
    let config = load_config();
    match server::start_daemonized(config) {
      Ok(_) => info!("Server daemon started"),
      Err(e) => {
        error!("Error daemonizing: {e}");
        exit(1);
      }
    }
  }
}

fn get_current_build_id() -> String {
  let manifest_path = Path::new(&get_working_dir())
    .join("steamapps")
    .join(format!("appmanifest_{}.acf", constants::GAME_ID));
  let manifest_data = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
    panic!(
      "Failed to read manifest file at '{}'",
      manifest_path.display()
    )
  });
  extract_build_id_from_manifest(&manifest_data).to_string()
}

fn get_latest_build_id() -> String {
  // Remove the cached file to force an updated response. This is done because `steamcmd` seems to
  // refuse to update information before querying the app_info even with `+app_info_update 1` or
  // `+@bCSForceNoCache 1`
  let app_info_file = Path::new("/home/steam/Steam/appcache/appinfo.vdf");
  fs::remove_file(app_info_file).unwrap_or_else(|e| match e.kind() {
    // AOK if it doesn't exist
    ErrorKind::NotFound => {}
    err_kind => {
      error!(
        "Failed to remove app_info file at '{}'! Error: {:?}",
        app_info_file.display(),
        err_kind
      );
      exit(1);
    }
  });

  // Now pull the latest app info
  let args = [
    "+@ShutdownOnFailedCommand 1",
    "+login anonymous",
    &format!("+app_info_print {}", constants::GAME_ID),
    "+quit",
  ];
  let arg_list = args.iter().map(|v| v.to_string()).collect::<Vec<_>>();
  let app_info_output = output_with_retries(&arg_list).expect("Failed to run steamcmd");
  assert!(app_info_output.status.success());

  let stdout = String::from_utf8(app_info_output.stdout).expect("steamcmd returned invalid UTF-8");
  extract_build_id_from_app_info(&stdout).to_string()
}

pub(crate) fn extract_build_id_from_manifest(manifest: &str) -> &str {
  let re = Regex::new(r"(buildid)\W+(\d+)\W").unwrap();
  // return group 2
  if let Some(captures) = re.captures(manifest) {
    captures.get(2).map_or("", |m| m.as_str())
  } else {
    panic!("Unexpected manifest format:\n{manifest}");
  }
}

pub(crate) fn extract_build_id_from_app_info(app_info: &str) -> &str {
  let re = Regex::new(r"depots.\n[\W\S]+public.\n\W+(buildid)\W+(\d+)\W").unwrap();
  // return group 2
  re.captures(app_info)
    .expect("Invalid App Info!")
    .get(2)
    .map_or("", |m| m.as_str())
}

#[cfg(test)]
mod tests {
  use super::*;

  use once_cell::sync::Lazy;

  use std::path::PathBuf;

  static TEST_ASSET_DIR: Lazy<PathBuf> = Lazy::new(|| {
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("tests")
      .join("assets")
  });

  const CURRENT_MANIFEST_FILENAME: &str = "example_current_app_manifest.txt";
  const CURRENT_APP_INFO_FILENAME: &str = "example_current_steamcmd_app_info.txt";
  const UPDATED_APP_INFO_FILENAME: &str = "example_updated_steamcmd_app_info.txt";

  const CURRENT_BUILD_ID: &str = "6246034";
  const UPDATED_BUILD_ID: &str = "6315977";

  fn read_sample_file(filename: &str) -> String {
    let filepath = TEST_ASSET_DIR.join(filename);
    fs::read_to_string(&filepath)
      .unwrap_or_else(|_| panic!("Sample file missing: '{}'", filepath.display()))
  }

  #[test]
  fn extracting_build_id_from_manifest() {
    let manifest_data = read_sample_file(CURRENT_MANIFEST_FILENAME);
    assert_eq!(
      extract_build_id_from_manifest(&manifest_data),
      CURRENT_BUILD_ID
    );
  }

  #[test]
  fn extracting_build_id_from_app_info() {
    let app_info_output = read_sample_file(CURRENT_APP_INFO_FILENAME);
    assert_eq!(
      extract_build_id_from_app_info(&app_info_output),
      CURRENT_BUILD_ID
    );
  }

  #[test]
  fn update_info() {
    let current_manifest_data = read_sample_file(CURRENT_MANIFEST_FILENAME);
    let current_app_info_output = read_sample_file(CURRENT_APP_INFO_FILENAME);
    let updated_app_info_output = read_sample_file(UPDATED_APP_INFO_FILENAME);

    // Verify updated info looks right
    let updated_update_info =
      UpdateInfo::new_testing(&current_manifest_data, &current_app_info_output);
    assert_eq!(
      updated_update_info,
      UpdateInfo {
        current_build_id: CURRENT_BUILD_ID.to_string(),
        latest_build_id: CURRENT_BUILD_ID.to_string()
      }
    );
    assert!(!updated_update_info.update_available());

    // Verify that info indicating an update looks right
    let pending_update_info =
      UpdateInfo::new_testing(&current_manifest_data, &updated_app_info_output);
    assert_eq!(
      pending_update_info,
      UpdateInfo {
        current_build_id: CURRENT_BUILD_ID.to_string(),
        latest_build_id: UPDATED_BUILD_ID.to_string()
      }
    );
    assert!(pending_update_info.update_available());
  }
}

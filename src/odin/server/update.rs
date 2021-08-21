use log::{debug, error, info};

use std::{fs, io::ErrorKind, path::Path, process::exit};

use crate::{
  constants, files::config::load_config, server, steamcmd::steamcmd_command, utils::get_working_dir,
};

#[derive(Clone, Debug, PartialEq)]
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

  // pub fn current_build_id(&self) -> &str {
  //   &self.current_build_id
  // }

  // pub fn latest_build_id(&self) -> &str {
  //   &self.latest_build_id
  // }
}

impl Default for UpdateInfo {
  fn default() -> Self {
    Self::new()
  }
}

pub fn update_is_available() -> bool {
  let info = UpdateInfo::new();
  debug!("{:#?}", info);

  info.update_available()
}

pub fn update_server() {
  // Shutdown the server if it's running
  let server_was_running = server::is_running();
  if server_was_running {
    server::blocking_shutdown();
  }

  // Update the installation
  if let Err(e) = server::install(constants::GAME_ID) {
    error!("Failed to install server: {}", e);
    exit(1);
  }

  // Bring the server up if it was running before
  if server_was_running {
    let config = load_config();
    match server::start_daemonized(config) {
      Ok(_) => info!("Server daemon started"),
      Err(e) => {
        error!("Error daemonizing: {}", e);
        exit(1);
      }
    }
  }
}

fn get_current_build_id() -> String {
  let manifest_path = Path::new(&get_working_dir())
    .join("steamapps")
    .join(&format!("appmanifest_{}.acf", constants::GAME_ID));
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
  fs::remove_file(&app_info_file).unwrap_or_else(|e| match e.kind() {
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
  let args = &[
    "+@ShutdownOnFailedCommand 1",
    "+login anonymous",
    &format!("+app_info_print {}", constants::GAME_ID),
    "+quit",
  ];
  let mut steamcmd = steamcmd_command();
  let app_info_output = steamcmd
    .args(args)
    .output()
    .expect("Failed to run steamcmd");
  assert!(app_info_output.status.success());

  let stdout = String::from_utf8(app_info_output.stdout).expect("steamcmd returned invalid UTF-8");
  extract_build_id_from_app_info(&stdout).to_string()
}

fn extract_build_id_from_manifest(manifest: &str) -> &str {
  for line in manifest.lines() {
    if line.trim().starts_with("\"buildid\"") {
      return split_vdf_key_val(line).1;
    }
  }

  panic!("Unexpected manifest format:\n{}", manifest);
}

fn extract_build_id_from_app_info(app_info: &str) -> &str {
  let mut lines = app_info.lines();
  for line in &mut lines {
    if line.trim() == "\"public\"" {
      break;
    }
  }

  assert_eq!(
    lines.next().map(|line| line.trim()),
    Some("{"),
    "Invalid app info"
  );
  let build_id_line = lines
    .next()
    .unwrap_or_else(|| panic!("Invalid app info format:\n{}", app_info))
    .trim();
  assert!(build_id_line.starts_with("\"buildid\""), "Invalid app info");

  split_vdf_key_val(build_id_line).1
}

// Note: This is super brittle and will fail if there is whitespace within the key or value _or_ if
// there are escaped " at the end of the key or value
fn split_vdf_key_val(vdf_pair: &str) -> (&str, &str) {
  let mut pieces = vdf_pair.trim().split_whitespace();
  let key = pieces.next().expect("Missing vdf key").trim_matches('"');
  let val = pieces.next().expect("Missing vdf val").trim_matches('"');

  (key, val)
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

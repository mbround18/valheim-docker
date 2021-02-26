use clap::ArgMatches;
use log::{debug, error, info};

use std::{fs, path::Path, process::exit};

use crate::{
  constants, files::config::load_config, server, steamcmd::steamcmd_command, utils::get_working_dir,
};

enum Action {
  Check,
  Force,
  Regular,
}

impl Action {
  fn new(check: bool, force: bool) -> Self {
    match (check, force) {
      (true, true) => panic!("`check` and `force` are mutually exlusive!"),
      (true, false) => Self::Check,
      (false, true) => Self::Force,
      (false, false) => Self::Regular,
    }
  }
}

pub fn invoke(args: &ArgMatches) {
  info!("Checking for updates");

  if !server::is_installed() {
    error!("Failed to find server executable. Can't update!");
    exit(1);
  }

  let dry_run = args.is_present("dry_run");
  let check = args.is_present("check");
  let force = args.is_present("force");

  let server_running = server::is_running();
  let update_available = update_is_available();
  if update_available {
    info!("A server update is available!");
  } else {
    info!("No server updates found");
  }

  match Action::new(check, force) {
    Action::Check => match (dry_run, update_available) {
      (true, true) => {
        info!("Dry run: An update is available. This would exit with 0 to indicate this.")
      }
      (true, false) => {
        info!("Dry run: No update is available. This would exit with 1 to indicate this.")
      }
      // 0 indicates that an update is available
      (false, true) => exit(0),
      // TODO: should we do a value other than 1 here, and if we do then what value?
      // 1 indicates the server is up to date
      (false, false) => exit(1),
    },
    Action::Force => match (dry_run, server_running) {
      (true, true) => info!("Dry run: Server would be shutdown, updated, and brought back online"),
      (true, false) => info!("Dry run: The server is offline and would be updated"),
      _ => {
        debug!("Force updating!");
        update_server();
      }
    },
    Action::Regular => {
      if dry_run {
        match (server_running, update_available) {
          (true, true) => info!(
            "Dry run: An update is available and the server is ONLINE. \
                    The server would be shutdown, updated, and brought back online."
          ),
          (false, true) => info!(
            "Dry run: An update is available and the server is OFFLINE. \
                    The server would be updated."
          ),
          (_, false) => info!("Dry run: No update is available. Nothing to do."),
        }
      } else if update_available {
        debug!("Updating the installation!");
        update_server()
      }
    }
  }
}

fn update_server() {
  // Shutdown the server if it's running
  let server_was_running = server::is_running();
  if server_was_running {
    server::send_shutdown();
    server::wait_for_exit();
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

fn update_is_available() -> bool {
  let latest_buildid = get_latest_buildid();
  let current_buildid = get_current_buildid();
  debug!(
    "latest buildid: {}, current buildid: {}",
    latest_buildid, current_buildid
  );

  latest_buildid != current_buildid
}

fn get_current_buildid() -> String {
  let manifest_path = Path::new(&get_working_dir())
    .join("steamapps")
    .join("appmanifest_896660.acf");
  let manifest_data = fs::read_to_string(manifest_path).expect("Failed to read manifest file");
  extract_buildid_from_manifest(&manifest_data).to_string()
}

fn get_latest_buildid() -> String {
  let args = &[
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
  extract_buildid_from_app_info(&stdout).to_string()
}

fn extract_buildid_from_manifest(manifest: &str) -> &str {
  let mut lines = manifest.lines();
  let build_id_line = loop {
    let line = lines.next().unwrap().trim();

    if line.starts_with("\"buildid\"") {
      break line;
    }
  };

  split_vdf_key_val(build_id_line).1
}

fn extract_buildid_from_app_info(app_info: &str) -> &str {
  let mut lines = app_info.lines();
  while let Some(line) = lines.next() {
    if line.trim() == "\"public\"" {
      break;
    }
  }

  assert_eq!(lines.next().map(|line| line.trim()), Some("{"));
  let build_id_line = lines.next().unwrap().trim();
  assert!(build_id_line.starts_with("\"buildid\""));

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

  #[test]
  fn extracting_buildid_from_manifest() {
    let sample_file = TEST_ASSET_DIR.join("example_app_manifest.txt");
    let manifest_data = fs::read_to_string(sample_file).expect("Sample manifest file missing");

    assert_eq!(extract_buildid_from_manifest(&manifest_data), "6246034");
  }

  #[test]
  fn extracting_buildid_from_app_info() {
    let sample_file = TEST_ASSET_DIR.join("example_steamcmd_app_info.txt");
    let app_info_output =
      fs::read_to_string(sample_file).expect("Sample app info output file missing");

    assert_eq!(extract_buildid_from_app_info(&app_info_output), "6246034");
  }
}

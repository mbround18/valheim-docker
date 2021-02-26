use clap::ArgMatches;
use log::{debug, error, info};

use std::process::exit;

use crate::{constants, files::config::load_config, server};

enum Action {
  Check,
  Force,
  Regular,
}

impl Action {
  fn new(check: bool, force: bool) -> Self {
    assert!(
      !(check && force),
      "`check` and `force` are mutually exlusive!"
    );

    if check {
      Self::Check
    } else if force {
      Self::Force
    } else {
      Self::Regular
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
    Action::Check => {
      if dry_run {
        if update_available {
          info!("Dry run: An update is available. This would exit with 0 to indicate this.");
        } else {
          info!("Dry run: No update is available. This would exit with 1 to indicate this.");
        }
      } else if !update_available {
        // TODO: should we do a value other than 1 here, and if we do then what value?
        // 0 exit code indicates there is an update while 1 indicates the server is up to date
        exit(1);
      }
    }
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
  // back up if it was before
  let server_was_running = server::is_running();

  // Shutdown the server if it's running
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
  get_latest_buildid() != get_current_buildid()
}

fn get_current_buildid() -> String {
  // TODO: Can parse this from the app manifest
  todo!();
}

fn get_latest_buildid() -> String {
  // TODO: can parse this from the program output of the one command
  todo!();
}

use clap::ArgMatches;
use log::{debug, error, info};

use std::process::exit;

use crate::server;

const EXIT_NO_UPDATE_AVAILABLE: i32 = 0;
const EXIT_UPDATE_AVAILABLE: i32 = 1;

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
    error!(
      "Failed to find server executable. Can't update! If the server isn't installed yet then you \
        likely need to run `odin install`."
    );
    exit(1);
  }

  let dry_run = args.is_present("dry_run");
  let check = args.is_present("check");
  let force = args.is_present("force");

  let server_running = server::is_running();
  let update_available = server::update_is_available();
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
      (false, true) => exit(EXIT_NO_UPDATE_AVAILABLE),
      // TODO: should we do a value other than 1 here, and if we do then what value?
      (false, false) => exit(EXIT_UPDATE_AVAILABLE),
    },
    Action::Force => match (dry_run, server_running) {
      (true, true) => info!("Dry run: Server would be shutdown, updated, and brought back online"),
      (true, false) => info!("Dry run: The server is offline and would be updated"),
      _ => {
        debug!("Force updating!");
        server::update_server();
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
        server::update_server()
      }
    }
  }
}

use clap::ArgMatches;
use log::{debug, info};

use std::process::exit;

enum Action {
  Check,
  Force,
  Regular,
}

impl Action {
  fn new(check: bool, force: bool) -> Self {
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

  let dry_run = args.is_present("dry_run");
  let check = args.is_present("check");
  let force = args.is_present("force");

  let update_available = update_is_available();
  if update_available {
    info!("A server update is available!");
  } else {
    info!("No server updates found");
  }

  match Action::new(check, force) {
    Action::Check => {
      if !dry_run && !update_available {
        // 0 indicates there is an update while 1 indicates the server is up to date
        exit(1);
      }
    }
    Action::Force => {
      debug!("Force updating!");
      update_server()
    }
    Action::Regular => {
      if update_available {
        debug!("An update is available. Updating!");
        update_server()
      }
    }
  }
}

fn update_server() {
  // TODO: Check if the server is up, if it is then shut it down, apply the update, and bring it
  // back up if it was before
  todo!()
}

fn update_is_available() -> bool {
  get_latest_buildid() != get_current_buildid()
}

fn get_current_buildid() -> String {
  // TODO: Can parse this from the app manifest
  todo!()
}

fn get_latest_buildid() -> String {
  // TODO: can parse this from the program output of the one command
  todo!()
}

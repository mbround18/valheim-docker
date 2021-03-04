use clap::ArgMatches;
use log::{debug, error, info};

use std::process::exit;

use crate::server;

const EXIT_NO_UPDATE_AVAILABLE: i32 = 0;
const EXIT_UPDATE_AVAILABLE: i32 = 1;

enum UpdateAction {
  Check,
  Force,
  Regular,
}

impl UpdateAction {
  fn new(check: bool, force: bool) -> Self {
    match (check, force) {
      (true, true) => panic!("`check` and `force` are mutually exlusive!"),
      (true, false) => Self::Check,
      (false, true) => Self::Force,
      (false, false) => Self::Regular,
    }
  }
}

enum RunAction {
  Real,
  Dry,
}

enum UpdateState {
  Pending,
  UpToDate,
}

impl UpdateState {
  fn new() -> Self {
    if server::update_is_available() {
      Self::Pending
    } else {
      Self::UpToDate
    }
  }

  fn as_exit_code(&self) -> i32 {
    match self {
      Self::UpToDate => EXIT_NO_UPDATE_AVAILABLE,
      Self::Pending => EXIT_UPDATE_AVAILABLE,
    }
  }
}

enum ServerState {
  Running,
  Stopped,
}

impl ServerState {
  fn new() -> Self {
    if server::is_running() {
      Self::Running
    } else {
      Self::Stopped
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

  let dry_run = if args.is_present("dry_run") {
    RunAction::Dry
  } else {
    RunAction::Real
  };
  let check = args.is_present("check");
  let force = args.is_present("force");

  let server_state = ServerState::new();
  let update_state = UpdateState::new();
  match update_state {
    UpdateState::Pending => info!("A server update is available!"),
    UpdateState::UpToDate => info!("No server updates found"),
  }

  match UpdateAction::new(check, force) {
    UpdateAction::Check => update_check(dry_run, update_state),
    UpdateAction::Force => update_force(dry_run, server_state),
    UpdateAction::Regular => update_regular(dry_run, server_state, update_state),
  }
}

fn update_check(dry_run: RunAction, update_state: UpdateState) {
  match (dry_run, update_state) {
    (RunAction::Dry, UpdateState::Pending) => {
      info!("Dry run: An update is available. This would exit with 0 to indicate this.")
    }
    (RunAction::Dry, UpdateState::UpToDate) => {
      info!("Dry run: No update is available. This would exit with 1 to indicate this.")
    }
    (_, update_state) => exit(update_state.as_exit_code()),
  }
}

fn update_force(dry_run: RunAction, server_state: ServerState) {
  match (dry_run, server_state) {
    (RunAction::Dry, ServerState::Running) => {
      info!("Dry run: Server would be shutdown, updated, and brought back online")
    }
    (RunAction::Dry, ServerState::Stopped) => {
      info!("Dry run: The server is offline and would be updated")
    }
    _ => {
      debug!("Force updating!");
      server::update_server();
    }
  }
}

fn update_regular(dry_run: RunAction, server_state: ServerState, update_state: UpdateState) {
  match (dry_run, server_state, update_state) {
    (RunAction::Dry, ServerState::Running, UpdateState::Pending) => {
      info!(
        "Dry run: An update is available and the server is ONLINE. The server would be shutdown \
          updated, and brought back online."
      )
    }
    (RunAction::Dry, ServerState::Stopped, UpdateState::Pending) => {
      info!(
        "Dry run: An update is available and the server is OFFLINE. The server would be updated."
      )
    }
    (RunAction::Dry, _, UpdateState::UpToDate) => {
      info!("Dry run: No update is available. Nothing to do.")
    }
    (_, _, UpdateState::Pending) => {
      debug!("Updating the installation!");
      server::update_server()
    }
    _ => debug!("No update available, nothing to do!"),
  }
}

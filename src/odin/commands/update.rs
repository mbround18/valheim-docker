use log::{debug, error, info};

use std::process::exit;

use crate::server;

const EXIT_NO_UPDATE_AVAILABLE: i32 = 10;
const EXIT_UPDATE_AVAILABLE: i32 = 0;

enum UpdateAction {
  Check,
  Force,
  Regular,
}

impl UpdateAction {
  fn new(check: bool, force: bool) -> Self {
    match (check, force) {
      (true, true) => panic!("`check` and `force` are mutually exclusive!"),
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

#[derive(Clone, Copy)]
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

pub fn invoke(dry_run: bool, check: bool, force: bool) {
  info!("Checking for updates");

  if !server::is_installed() {
    error!(
      "Failed to find server executable. Can't update! If the server isn't installed yet then you \
        likely need to run `odin install`."
    );
    exit(1);
  }

  let run_action = if dry_run {
    RunAction::Dry
  } else {
    RunAction::Real
  };

  let server_state = ServerState::new();
  let update_state = UpdateState::new();
  match update_state {
    UpdateState::Pending => info!("A server update is available!"),
    UpdateState::UpToDate => info!("No server updates found"),
  }

  match UpdateAction::new(check, force) {
    UpdateAction::Check => update_check(run_action, update_state),
    UpdateAction::Force => update_force(run_action, server_state),
    UpdateAction::Regular => update_regular(run_action, server_state, update_state),
  }
}

fn update_check(run_action: RunAction, update_state: UpdateState) {
  match (run_action, update_state) {
    (RunAction::Dry, UpdateState::Pending) => {
      info!(
        "Dry run: An update is available. This would exit with {} to indicate this.",
        update_state.as_exit_code()
      )
    }
    (RunAction::Dry, UpdateState::UpToDate) => {
      info!(
        "Dry run: No update is available. This would exit with {} to indicate this.",
        update_state.as_exit_code()
      )
    }
    (_, update_state) => exit(update_state.as_exit_code()),
  }
}

fn update_force(run_action: RunAction, server_state: ServerState) {
  match (run_action, server_state) {
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

fn update_regular(run_action: RunAction, server_state: ServerState, update_state: UpdateState) {
  match (run_action, server_state, update_state) {
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

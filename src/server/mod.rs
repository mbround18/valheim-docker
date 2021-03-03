mod install;
mod shutdown;
mod startup;
mod update;
mod utils;

pub use crate::server::{
  install::{install, is_installed},
  shutdown::{blocking_shutdown, send_shutdown_signal, wait_for_exit},
  startup::{start, start_daemonized},
  update::{update_is_available, update_server, UpdateInfo},
  utils::is_running,
};

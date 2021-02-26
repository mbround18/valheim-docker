use clap::{load_yaml, App};
use log::{debug, info, LevelFilter, SetLoggerError};

use crate::executable::handle_exit_status;
use crate::logger::OdinLogger;
use crate::utils::fetch_env;
mod commands;
mod errors;
mod executable;
mod files;
mod logger;
mod messages;
mod notifications;
mod steamcmd;
mod utils;

use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;

const VERSION: &str = env!("CARGO_PKG_VERSION");
static LOGGER: OdinLogger = OdinLogger;
static GAME_ID: i64 = 896660;

fn setup_logger(debug: bool) -> Result<(), SetLoggerError> {
  let level = if debug {
    LevelFilter::Debug
  } else {
    LevelFilter::Info
  };
  let result = log::set_logger(&LOGGER).map(|_| log::set_max_level(level));
  debug!("Debugging set to {}", debug.to_string());
  result
}

fn main() {
  // The YAML file is found relative to the current file, similar to how modules are found
  let yaml = load_yaml!("cli.yaml");
  let app = App::from(yaml).version(VERSION);
  let matches = app.get_matches();
  let debug_mode = matches.is_present("debug") || fetch_env("DEBUG_MODE", "0", false).eq("1");
  setup_logger(debug_mode).unwrap();
  if !debug_mode {
    info!("Run with DEBUG_MODE as 1 if you think there is an issue with Odin");
  }
  debug!("Debug mode enabled!");
  if let Some(ref configure_matches) = matches.subcommand_matches("configure") {
    debug!("Launching configure command...");
    commands::configure::invoke(configure_matches);
  };
  if let Some(ref _match) = matches.subcommand_matches("install") {
    debug!("Launching install command...");
    let result = commands::install::invoke(GAME_ID);
    handle_exit_status(result, "Successfully installed Valheim!".to_string())
  };
  if let Some(ref start_matches) = matches.subcommand_matches("start") {
    debug!("Launching start command...");
    commands::start::invoke(start_matches);
    NotificationEvent::Start(EventStatus::Successful).send_notification();
  };
  if let Some(ref stop_matches) = matches.subcommand_matches("stop") {
    debug!("Launching stop command...");
    NotificationEvent::Stop(EventStatus::Running).send_notification();
    commands::stop::invoke(stop_matches);
    NotificationEvent::Stop(EventStatus::Successful).send_notification();
  };
  if let Some(ref backup_matches) = matches.subcommand_matches("backup") {
    debug!("Launching backup command...");
    commands::backup::invoke(backup_matches);
  };
  if let Some(ref notify_matches) = matches.subcommand_matches("notify") {
    debug!("Launching notify command...");
    commands::notify::invoke(notify_matches);
  };
}

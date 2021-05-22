use clap::{load_yaml, App, AppSettings};
use log::debug;

use crate::executable::handle_exit_status;
use crate::utils::environment;
pub mod commands;
mod constants;
mod errors;
mod executable;
mod files;
mod logger;
mod messages;
mod mods;
mod notifications;
pub mod server;
mod steamcmd;
pub mod utils;

use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;

fn main() {
  // The YAML file is found relative to the current file, similar to how modules are found
  let yaml = load_yaml!("cli.yaml");
  let app = App::from(yaml)
    .version(constants::VERSION)
    .setting(AppSettings::SubcommandRequired);
  let matches = app.get_matches();
  let debug_mode = matches.is_present("debug") || environment::fetch_var("DEBUG_MODE", "0").eq("1");

  if 0_u32 == users::get_current_uid() && !matches.is_present("run_as_root") {
    panic!("\x1b[0;31m\n\nWoah! You cannot launch this program as root!\n\nIf this was intentional please rerun with --run-as-root\n\nYou might run into permission issues if you run this as root!\x1b[0m\n\n")
  }

  logger::initialize_logger(debug_mode).unwrap();
  debug!("Debug mode enabled!");
  if let Some((command_name, _)) = matches.subcommand() {
    debug!("Launching {} command...", command_name);
  };
  match matches.subcommand().expect("Subcommand is required") {
    ("configure", sub_m) => commands::configure::invoke(sub_m),
    ("install", _) => {
      let result = commands::install::invoke(constants::GAME_ID);
      handle_exit_status(result, "Successfully installed Valheim!".to_string())
    }
    ("start", sub_m) => {
      NotificationEvent::Start(EventStatus::Running).send_notification();
      commands::start::invoke(sub_m);
      NotificationEvent::Start(EventStatus::Successful).send_notification();
    }
    ("stop", sub_m) => {
      NotificationEvent::Stop(EventStatus::Running).send_notification();
      commands::stop::invoke(sub_m);
      NotificationEvent::Stop(EventStatus::Successful).send_notification();
    }
    ("backup", sub_m) => commands::backup::invoke(sub_m),
    ("notify", sub_m) => commands::notify::invoke(sub_m),
    ("update", sub_m) => commands::update::invoke(sub_m),
    ("mod:install", sub_m) => commands::install_mod::invoke(sub_m),
    ("status", sub_m) => commands::status::invoke(sub_m),
    _ => {
      panic!("No Command Launched!");
    } // Either no subcommand or one not tested for...
  }
}

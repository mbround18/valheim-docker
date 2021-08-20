use clap::{load_yaml, App, AppSettings};
use dotenv::dotenv;
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

fn main() {
  dotenv().ok();
  // The YAML file is found relative to the current file, similar to how modules are found
  let yaml = load_yaml!("cli.yaml");
  let app = App::from(yaml)
    .version(constants::VERSION)
    .setting(AppSettings::SubcommandRequired)
    .setting(AppSettings::ArgRequiredElseHelp);
  let matches = app.get_matches();
  let debug_mode = matches.is_present("debug") || environment::fetch_var("DEBUG_MODE", "0").eq("1");

  if 0_u32 == users::get_current_uid() && !matches.is_present("run_as_root") {
    panic!("\x1b[0;31m\n\nWoah! You cannot launch this program as root!\n\nIf this was intentional please rerun with --run-as-root\n\nYou might run into permission issues if you run this as root!\x1b[0m\n\n")
  }

  logger::initialize_logger(debug_mode).unwrap();
  if debug_mode {
    debug!("Debug mode enabled!");
  }
  let command_name = matches.subcommand();
  debug!("Launching {} command...", command_name.0);

  match matches.subcommand() {
    ("configure", sub_m) => commands::configure::invoke(sub_m.unwrap()),
    ("install", _) => handle_exit_status(
      commands::install::invoke(constants::GAME_ID),
      "Successfully installed Valheim!".to_string(),
    ),
    ("start", sub_m) => commands::start::invoke(sub_m.unwrap()),
    ("stop", sub_m) => commands::stop::invoke(sub_m.unwrap()),
    ("backup", sub_m) => commands::backup::invoke(sub_m.unwrap()),
    ("notify", sub_m) => commands::notify::invoke(sub_m.unwrap()),
    ("update", sub_m) => commands::update::invoke(sub_m.unwrap()),
    ("mod:install", sub_m) => commands::install_mod::invoke(sub_m.unwrap()),
    ("status", sub_m) => commands::status::invoke(sub_m.unwrap()),
    _ => {
      panic!("No Command Launched!");
    } // Either no subcommand or one not tested for...
  }
}

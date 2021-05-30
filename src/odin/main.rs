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
    .setting(AppSettings::SubcommandRequired);
  let matches = app.get_matches();
  let debug_mode = matches.is_present("debug") || environment::fetch_var("DEBUG_MODE", "0").eq("1");
  logger::initialize_logger(debug_mode).unwrap();
  debug!("Debug mode enabled!");
  if let Some((command_name, _)) = matches.subcommand() {
    debug!("Launching {} command...", command_name);
  };
  match matches.subcommand().expect("Subcommand is required") {
    ("configure", sub_m) => commands::configure::invoke(sub_m),
    ("install", _) => handle_exit_status(
      commands::install::invoke(constants::GAME_ID),
      "Successfully installed Valheim!".to_string(),
    ),
    ("start", sub_m) => commands::start::invoke(sub_m),
    ("stop", sub_m) => commands::stop::invoke(sub_m),
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

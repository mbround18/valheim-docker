use clap::Parser;
use dotenv::dotenv;
use log::debug;

use crate::executable::handle_exit_status;
mod cli;
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

  use cli::{Cli, Commands};
  let cli = Cli::parse();

  logger::initialize_logger(cli.debug).unwrap();

  if cli.debug {
    debug!("Debug mode enabled!");
  }

  match cli.commands {
    Commands::Configure {
      name,
      public,
      password,
      server_executable,
      world,
      port,
    } => commands::configure::Configuration::new(
      name,
      server_executable,
      port,
      world,
      password,
      public,
    )
    .invoke(),
    Commands::Install {} => handle_exit_status(
      commands::install::invoke(constants::GAME_ID),
      "Successfully installed Valheim!".to_string(),
    ),
    Commands::Start {} => commands::start::invoke(cli.dry_run),
    Commands::Stop {} => commands::stop::invoke(cli.dry_run),
    Commands::Backup {
      input_directory,
      output_file,
    } => commands::backup::invoke(input_directory, output_file),
    Commands::Update { check, force } => commands::update::invoke(cli.dry_run, check, force),
    Commands::Notify {
      title,
      message,
      webhook_url,
    } => commands::notify::invoke(title, message, webhook_url),
    Commands::ModInstall { url } => commands::install_mod::invoke(url),
    Commands::Status {
      json,
      local,
      address,
    } => commands::status::invoke(json, local, address),
  }

  // // The YAML file is found relative to the current file, similar to how modules are found
  // let yaml = load_yaml!("cli.yaml");
  // let app = App::from(yaml)
  //   .version(constants::VERSION)
  //   .setting(AppSettings::SubcommandRequired)
  //   .setting(AppSettings::ArgRequiredElseHelp);
  // let matches = app.get_matches();
  // let debug_mode = matches.is_present("debug") || environment::fetch_var("DEBUG_MODE", "0").eq("1");
  //
  // if 0_u32 == users::get_current_uid() && !matches.is_present("run_as_root") {
  //   panic!("\x1b[0;31m\n\nWoah! You cannot launch this program as root!\n\nIf this was intentional please rerun with --run-as-root\n\nYou might run into permission issues if you run this as root!\x1b[0m\n\n")
  // }
  //
  // logger::initialize_logger(debug_mode).unwrap();
  // if debug_mode {
  //   debug!("Debug mode enabled!");
  // }
  // let command_name = matches.subcommand();
  // debug!("Launching {} command...", command_name.0);
  //
  // match matches.subcommand() {
  //   ("configure", sub_m) => commands::configure::invoke(sub_m.unwrap()),
  //   ("install", _) => handle_exit_status(
  //     commands::install::invoke(constants::GAME_ID),
  //     "Successfully installed Valheim!".to_string(),
  //   ),
  //   ("start", sub_m) => commands::start::invoke(sub_m.unwrap()),
  //   ("stop", sub_m) => commands::stop::invoke(sub_m.unwrap()),
  //   ("backup", sub_m) => commands::backup::invoke(sub_m.unwrap()),
  //   ("notify", sub_m) => commands::notify::invoke(sub_m.unwrap()),
  //   ("update", sub_m) => commands::update::invoke(sub_m.unwrap()),
  //   ("mod:install", sub_m) => commands::install_mod::invoke(sub_m.unwrap()),
  //   ("status", sub_m) => commands::status::invoke(sub_m.unwrap()),
  //   _ => {
  //     panic!("No Command Launched!");
  //   } // Either no subcommand or one not tested for...
  // }
}

use clap::Parser;
use dotenv::dotenv;
use log::debug;

use crate::cli::{Cli, Commands};
use commands::configure::Configuration;

use crate::commands::configure::Modifiers;
use crate::executable::handle_exit_status;
use crate::logger::debug_mode;
use crate::messages::about;

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
pub mod traits;
pub mod utils;

#[tokio::main]
async fn main() {
  dotenv().ok();
  let cli = initialize_cli();
  initialize_logger(&cli);

  if cli.debug {
    debug!("Debug mode enabled!");
  }

  handle_commands(cli).await;
}

fn initialize_cli() -> Cli {
  Cli::parse()
}

fn initialize_logger(cli: &Cli) {
  logger::initialize_logger(cli.debug || debug_mode()).unwrap();
}

async fn handle_commands(cli: Cli) {
  match cli.commands {
    Commands::Configure {
      name,
      public,
      password,
      server_executable,
      world,
      port,
      modifiers,
      preset,
      set_key,
      save_interval,
    } => Configuration::new(
      name,
      server_executable,
      port,
      world,
      password,
      public.eq("1"),
      preset,
      modifiers.map(|m| {
        m.split(',')
          .map(|modifier| Modifiers::from(modifier.to_string()))
          .collect()
      }),
      set_key,
      save_interval,
      std::env::var("STEAM_HOME_DIRNAME").unwrap_or("/home/steam".to_string()),
    )
    .invoke()
    .await
    .expect("Failed to configure server"),
    Commands::Install {} => handle_exit_status(
      commands::install::invoke(constants::GAME_ID),
      "Successfully installed Valheim!".to_string(),
    ),
    Commands::Start {} => commands::start::invoke(cli.dry_run, cli.pause_on_idle_s),
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
    Commands::About {} => about(env!("GIT_HASH")),
    Commands::Logs { lines, watch } => commands::logs::invoke(lines, watch).await,
  }
}

#[cfg(test)]
mod tests {
  // use super::*;

  use clap::CommandFactory;

  use crate::cli::Cli;

  #[test]
  fn asserts() {
    Cli::command().debug_assert();
  }
}

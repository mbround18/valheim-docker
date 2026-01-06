use clap::Parser;
use dotenv::dotenv;

use crate::cli::{Cli, Commands, LevelArg};
use commands::configure::Configuration;

use crate::commands::configure::Modifiers;
use crate::executable::handle_exit_status;
use crate::messages::about;

mod cli;
pub mod commands;
mod constants;
mod errors;
mod executable;
mod files;
pub mod log_filters;
mod messages;
mod mods;
mod notifications;
pub mod server;
mod steamcmd;
pub mod traits;
pub mod utils;

use shared::init_logging_and_tracing;

#[tokio::main]
async fn main() {
  dotenv().ok();
  let cli = initialize_cli();
  // initialize_logger(&cli);

  init_logging_and_tracing().expect("Failed to initialize logging and tracing");

  handle_commands(cli).await;
}

fn initialize_cli() -> Cli {
  Cli::parse()
}

async fn handle_commands(cli: Cli) {
  match cli.commands {
    Commands::Log { message, level } => match level {
      // Print directly to avoid re-entry into the tracing/log bridge which can duplicate output.
      LevelArg::Error => eprintln!("{}", message),
      LevelArg::Warn => println!("WARN odin: {}", message),
      LevelArg::Info => println!("INFO odin: {}", message),
      LevelArg::Debug => println!("DEBUG odin: {}", message),
      LevelArg::Trace => println!("TRACE odin: {}", message),
    },
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
    )
    .invoke()
    .await
    .expect("Failed to configure server"),
    Commands::Install => handle_exit_status(
      commands::install::invoke(constants::GAME_ID),
      "Successfully installed Valheim!".to_string(),
    ),
    Commands::Start => commands::start::invoke(cli.dry_run),
    Commands::Stop => commands::stop::invoke(cli.dry_run),
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
    Commands::ModInstall { from_var, url } => commands::install_mod::invoke(url, from_var).await,
    Commands::Status {
      json,
      local,
      address,
    } => commands::status::invoke(json, local, address),
    Commands::About => about(env!("GIT_HASH")),
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

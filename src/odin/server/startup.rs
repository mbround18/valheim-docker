use std::fs::File;
use std::{io, process::Child};

use daemonize::{Daemonize, Error};
use log::{debug, error, info};

use crate::executable::parse_command_args;
use crate::mods::bepinex::BepInExEnvironment;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::common_paths::game_directory;
use crate::{
  constants,
  executable::{create_execution, execute_mut},
  files::{config::ValheimArguments, create_file},
  messages,
  utils::environment,
};

type CommandResult = io::Result<Child>;

fn create_log_files() -> Result<(File, File), Error> {
  let game_dir = game_directory();
  let stdout = create_file(format!("{game_dir}/logs/valheim_server.log").as_str());
  let stderr = create_file(format!("{game_dir}/logs/valheim_server.err").as_str());
  Ok((stdout, stderr))
}
pub fn start_daemonized(config: ValheimArguments) -> Result<CommandResult, Error> {
  debug!("Starting server daemonized...");
  let (stdout, stderr) = create_log_files().unwrap();
  let command = start(config);
  Daemonize::new()
    .working_directory(game_directory())
    .user("steam")
    .stdout(stdout)
    .stderr(stderr)
    .privileged_action(|| {
      let bepinex_env = BepInExEnvironment::new();
      if bepinex_env.is_installed() {
        info!("Server has been started with BepInEx! Keep in mind this may cause errors!!");
        messages::modding_disclaimer();
        debug!("{bepinex_env:#?}");
      }
      info!("Server has been started and Daemonize. It should be online shortly!");
      info!("Keep an eye out for 'Game server connected' in the log!");
      NotificationEvent::Start(EventStatus::Successful).send_notification(None);
      info!("(this indicates its online without any errors.)")
    })
    .privileged_action(|| command)
    .start()
}

pub fn start(config: ValheimArguments) -> CommandResult {
  let mut command = create_execution(&config.command);
  debug!("--------------------------------------------------------------------------------------------------------------");
  let (stdout, stderr) = create_log_files().unwrap();

  debug!("Launching With Args: \n{:#?}", &config);
  let base_command = command
    .env(
      "SteamAppId",
      // See https://www.reddit.com/r/valheim/comments/yvyxo8/trouble_with_the_dedicated_server/
      String::from("892970"), // fetch_var("APPID", "896660")
    )
    .current_dir(game_directory());

  base_command.stdout(stdout);
  base_command.stderr(stderr);
  debug!("Base Command: {base_command:#?}");

  // i want to error! on error and panic
  let mut args: Vec<String> = match config.clone().try_into() {
    Ok(a) => a,
    Err(e) => {
      error!("Error parsing configuration into command line arguments: {e}");
      Vec::new()
    }
  };

  if args.is_empty() {
    error!("No arguments were parsed! This is likely a configuration error!");
    error!("Please check your configuration with `odin configure --check`");
    error!("Exiting...");
    std::process::exit(1);
  }

  args = parse_command_args(args);

  base_command.args(&args as &[String]);

  debug!("Executable: {}", &config.command);
  info!("Launching Command...");
  let ld_library_path_value = environment::fetch_multiple_var(
    constants::LD_LIBRARY_PATH_VAR,
    format!("{}/linux64", game_directory()).as_str(),
  );

  let bepinex_env = BepInExEnvironment::new();
  if bepinex_env.is_installed() {
    info!("BepInEx detected! Switching to run with BepInEx...");
    info!("BepInEx Environment: \n{bepinex_env:#?}");
    bepinex_env.launch(command)
  } else {
    info!("Everything looks good! Running normally!");

    command.env(constants::LD_LIBRARY_PATH_VAR, ld_library_path_value);

    execute_mut(&mut command)
  }
}

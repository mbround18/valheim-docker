use daemonize::{Daemonize, DaemonizeError};
use log::{debug, info};

use std::{io, process::Child};

use crate::{
  constants,
  executable::create_execution,
  files::{create_file, ValheimArguments},
  messages,
  mods::bepinex,
  utils::{fetch_env, get_working_dir},
};

type CommandResult = io::Result<Child>;

pub fn start_daemonized(config: ValheimArguments) -> Result<CommandResult, DaemonizeError> {
  let stdout = create_file(format!("{}/logs/valheim_server.log", get_working_dir()).as_str());
  let stderr = create_file(format!("{}/logs/valheim_server.err", get_working_dir()).as_str());
  Daemonize::new()
    .working_directory(get_working_dir())
    .user("steam")
    .group("steam")
    .stdout(stdout)
    .stderr(stderr)
    .exit_action(|| {
      if bepinex::is_bepinex_installed() {
        info!("Server has been started with BepInEx! Keep in mind this may cause errors!!");
        messages::modding_disclaimer()
      }
      info!("Server has been started and Daemonized. It should be online shortly!");
      info!("Keep an eye out for 'Game server connected' in the log!");
      info!("(this indicates its online without any errors.)")
    })
    .privileged_action(move || start(&config))
    .start()
}

pub fn start(config: &ValheimArguments) -> CommandResult {
  let mut command = create_execution(&config.command);
  info!("--------------------------------------------------------------------------------------------------------------");
  let ld_library_path_value = fetch_env(
    constants::LD_LIBRARY_PATH_VAR,
    format!("{}/linux64", get_working_dir()).as_str(),
    true,
  );
  debug!("Setting up base command");
  let base_command = command
    .args(&[
      "-nographics",
      "-batchmode",
      "-port",
      &config.port.as_str(),
      "-name",
      &config.name.as_str(),
      "-world",
      &config.world.as_str(),
      "-password",
      &config.password.as_str(),
      "-public",
      &config.public.as_str(),
    ])
    .env("SteamAppId", fetch_env("APPID", "892970", false))
    .current_dir(get_working_dir());
  info!("Executable: {}", &config.command);
  info!("Launching Command...");

  if bepinex::is_bepinex_installed() {
    info!("BepInEx detected! Switching to run with BepInEx...");
    let bepinex_env = bepinex::build_environment();
    bepinex::invoke(base_command, &bepinex_env)
  } else {
    info!("Everything looks good! Running normally!");
    base_command
      .env(constants::LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .spawn()
  }
}

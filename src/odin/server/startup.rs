use daemonize::{Daemonize, DaemonizeError};
use log::{debug, info};

use std::{io, process::Child};

use crate::mods::bepinex::BepInExEnvironment;
use crate::utils::common_paths::saves_directory;
use crate::{
  constants,
  executable::create_execution,
  files::{create_file, ValheimArguments},
  messages,
  utils::{environment, get_working_dir},
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
      let bepinex_env = BepInExEnvironment::new();
      if bepinex_env.is_installed() {
        info!("Server has been started with BepInEx! Keep in mind this may cause odin.errors!!");
        messages::modding_disclaimer();
        debug!("{:#?}", bepinex_env);
      }
      info!("Server has been started and Daemonized. It should be online shortly!");
      info!("Keep an eye out for 'Game server connected' in the log!");
      info!("(this indicates its online without any odin.errors.)")
    })
    .privileged_action(move || start(&config))
    .start()
}

pub fn start(config: &ValheimArguments) -> CommandResult {
  let mut command = create_execution(&config.command);
  info!("--------------------------------------------------------------------------------------------------------------");
  let ld_library_path_value = environment::fetch_multiple_var(
    constants::LD_LIBRARY_PATH_VAR,
    format!("{}/linux64", get_working_dir()).as_str(),
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
      "-savedir",
      &saves_directory(),
    ])
    .env("SteamAppId", environment::fetch_var("APPID", "892970"))
    .current_dir(get_working_dir());
  info!("Executable: {}", &config.command);
  info!("Launching Command...");
  let bepinex_env = BepInExEnvironment::new();
  if bepinex_env.is_installed() {
    info!("BepInEx detected! Switching to run with BepInEx...");
    debug!("BepInEx Environment: \n{:#?}", bepinex_env);
    bepinex_env.launch(base_command)
  } else {
    info!("Everything looks good! Running normally!");
    base_command
      .env(constants::LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .spawn()
  }
}

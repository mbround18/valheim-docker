mod bepinex;

use crate::commands::start::bepinex::{build_environment, is_bepinex_installed};
use crate::executable::create_execution;
use crate::files::config::{config_file, read_config};
use crate::files::{create_file, ValheimArguments};
use crate::messages::modding_disclaimer;
use crate::utils::{fetch_env, get_working_dir};
use clap::ArgMatches;
use daemonize::Daemonize;
use log::{debug, error, info};
use std::process::{exit, Child};

const LD_LIBRARY_PATH_VAR: &str = "LD_LIBRARY_PATH";
const LD_PRELOAD_VAR: &str = "LD_PRELOAD";

fn exit_action() {
  if is_bepinex_installed() {
    info!("Server has been started with BepInEx! Keep in mind this may cause errors!!");
    modding_disclaimer()
  }
  info!("Server has been started and Daemonized. It should be online shortly!");
  info!("Keep an eye out for 'Game server connected' in the log!");
  info!("(this indicates its online without any errors.)")
}

fn spawn_server(config: &ValheimArguments) -> std::io::Result<Child> {
  let mut command = create_execution(&config.command);
  info!("--------------------------------------------------------------------------------------------------------------");
  let ld_library_path_value = fetch_env(
    LD_LIBRARY_PATH_VAR,
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

  if is_bepinex_installed() {
    info!("BepInEx detected! Switching to run with BepInEx...");
    let bepinex_env = build_environment();
    bepinex::invoke(base_command, &bepinex_env)
  } else {
    info!("Everything looks good! Running normally!");
    base_command
      .env(LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .spawn()
  }
}

pub fn invoke(args: &ArgMatches) {
  info!("Setting up start scripts...");
  debug!("Loading config file...");
  let config = config_file();
  let config_content: ValheimArguments = read_config(config);
  debug!("Checking password compliance...");
  if config_content.password.len() < 5 {
    error!("The supplied password is too short! It much be 5 characters or greater!");
    exit(1)
  }
  let dry_run: bool = args.is_present("dry_run");
  debug!("Dry run condition: {}", dry_run);
  info!("Looking for burial mounds...");
  if !dry_run {
    let stdout = create_file(format!("{}/logs/valheim_server.log", get_working_dir()).as_str());
    let stderr = create_file(format!("{}/logs/valheim_server.err", get_working_dir()).as_str());
    let daemonize = Daemonize::new()
      .working_directory(get_working_dir())
      .user("steam")
      .group("steam")
      .stdout(stdout)
      .stderr(stderr)
      .exit_action(exit_action)
      .privileged_action(move || spawn_server(&config_content));

    match daemonize.start() {
      Ok(_) => info!("Success, daemonized"),
      Err(e) => error!("Error, {}", e),
    }
  } else {
    info!(
      "This command would have launched\n{} -nographics -batchmode -port {} -name {} -world {} -password {} -public {}",
      &config_content.command,
      &config_content.port,
      &config_content.name,
      &config_content.world,
      &config_content.password,
      &config_content.public,
    )
  }
}

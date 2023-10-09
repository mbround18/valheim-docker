use std::process::exit;
use std::{io, process::Child};

use daemonize::{Daemonize, Error};
use log::{debug, error, info};

use crate::mods::bepinex::BepInExEnvironment;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::common_paths::{game_directory, saves_directory};
use crate::utils::environment::fetch_var;
use crate::{
  constants,
  executable::create_execution,
  files::{config::ValheimArguments, create_file},
  messages,
  utils::environment,
};

type CommandResult = io::Result<Child>;

pub fn start_daemonized(config: ValheimArguments) -> Result<CommandResult, Error> {
  let stdout = create_file(format!("{}/logs/valheim_server.log", game_directory()).as_str());
  let stderr = create_file(format!("{}/logs/valheim_server.err", game_directory()).as_str());
  Daemonize::new()
    .working_directory(game_directory())
    .user("steam")
    .group("steam")
    .stdout(stdout)
    .stderr(stderr)
    .privileged_action(|| {
      let bepinex_env = BepInExEnvironment::new();
      if bepinex_env.is_installed() {
        info!(target: "server_startup","Server has been started with BepInEx! Keep in mind this may cause errors!!");
        messages::modding_disclaimer();
        debug!(target: "server_startup","{:#?}", bepinex_env);
      }
      info!(target: "server_startup","Server has been started and Daemonize. It should be online shortly!");
      info!(target: "server_startup","Keep an eye out for 'Game server connected' in the log!");
      NotificationEvent::Start(EventStatus::Successful).send_notification();
      info!(target: "server_startup","(this indicates its online without any errors.)")
    })
    .privileged_action(move || start(&config))
    .start()
}

pub fn start(config: &ValheimArguments) -> CommandResult {
  let mut command = create_execution(&config.command);
  info!(target: "server_startup","--------------------------------------------------------------------------------------------------------------");
  let ld_library_path_value = environment::fetch_multiple_var(
    constants::LD_LIBRARY_PATH_VAR,
    format!("{}/linux64", game_directory()).as_str(),
  );
  debug!(target: "server_startup","Setting up base command");
  debug!("Launching With Args: \n{:#?}", &config);
  let mut base_command = command
    // Extra launch arguments
    .arg(fetch_var(
      "SERVER_EXTRA_LAUNCH_ARGS",
      "-nographics -batchmode",
    ))
    // Required vars
    .args([
      "-port",
      config.port.as_str(),
      "-name",
      config.name.as_str(),
      "-world",
      config.world.as_str(),
      "-public",
      config.public.as_str(),
    ])
    .arg(if let Some(set_key) = &config.set_key {
      format!("-setkey {}", set_key)
    } else {
      String::new()
    })
    .arg(if let Some(preset) = &config.preset {
      format!("-preset {}", preset)
    } else {
      String::new()
    })
    .arg(if let Some(modifiers) = &config.modifiers {
      modifiers
        .iter()
        .map(|modifier| format!("-modifier {} {}", modifier.name, modifier.value))
        .collect::<Vec<String>>()
        .join(" ")
        .to_string()
    } else {
      String::new()
    })
    .env("SteamAppId", fetch_var("APPID", "892970"))
    .current_dir(game_directory());

  let is_public = config.public.eq("1");
  let is_vanilla = fetch_var("TYPE", "vanilla").eq_ignore_ascii_case("vanilla");
  let no_password = config.password.is_empty();

  // If no password env variable
  if !is_public && !is_vanilla && no_password {
    debug!(target: "server_startup","No password found, skipping password flag.")
  } else if no_password && (is_public || is_vanilla) {
    error!("Cannot run you server with no password! PUBLIC must be 0 and cannot be a Vanilla type server.");
    exit(1)
  } else {
    debug!(target: "server_startup","Password found, adding password flag.");
    base_command = base_command.args(["-password", config.password.as_str()]);
  }

  if fetch_var("ENABLE_CROSSPLAY", "0").eq("1") {
    info!("Launching with Crossplay! <3");
    base_command = base_command.arg("-crossplay")
  } else {
    debug!("No Crossplay Enabled!")
  }

  // Tack on save dir at the end.
  base_command = base_command.args(["-savedir", &saves_directory()]);

  info!(target: "server_startup","Executable: {}", &config.command);
  info!(target: "server_startup","Launching Command...");
  let bepinex_env = BepInExEnvironment::new();
  if bepinex_env.is_installed() {
    info!(target: "server_startup","BepInEx detected! Switching to run with BepInEx...");
    debug!(target: "server_startup","BepInEx Environment: \n{:#?}", bepinex_env);
    bepinex_env.launch(base_command)
  } else {
    info!(target: "server_startup","Everything looks good! Running normally!");
    base_command
      .env(constants::LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .spawn()
  }
}

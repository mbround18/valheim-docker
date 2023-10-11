use std::{io, process::Child};
use std::process::exit;

use daemonize::{Daemonize, Error};
use log::{debug, error, info};

use crate::{
  constants,
  executable::create_execution,
  files::{config::ValheimArguments, create_file},
  messages,
  utils::environment,
};
use crate::mods::bepinex::BepInExEnvironment;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::common_paths::{game_directory, saves_directory};
use crate::utils::environment::fetch_var;

type CommandResult = io::Result<Child>;

pub fn start_daemonized(config: ValheimArguments) -> Result<CommandResult, Error> {
  debug!("Starting server daemonized...");
  let stdout = create_file(format!("{}/logs/valheim_server.log", game_directory()).as_str());
  let stderr = create_file(format!("{}/logs/valheim_server.err", game_directory()).as_str());
  let command = start(config);
  Daemonize::new()
    .working_directory(game_directory())
    .user("steam")
    .group("steam")
    .stdout(stdout)
    .stderr(stderr)
    .privileged_action(|| {
      let bepinex_env = BepInExEnvironment::new();
      if bepinex_env.is_installed() {
        info!("Server has been started with BepInEx! Keep in mind this may cause errors!!");
        messages::modding_disclaimer();
        debug!("{:#?}", bepinex_env);
      }
      info!("Server has been started and Daemonize. It should be online shortly!");
      info!("Keep an eye out for 'Game server connected' in the log!");
      NotificationEvent::Start(EventStatus::Successful).send_notification();
      info!("(this indicates its online without any errors.)")
    })
      .privileged_action(move || command)
    .start()
}

pub fn start(config: ValheimArguments) -> CommandResult {
  let mut command = create_execution(&config.command);

  debug!("--------------------------------------------------------------------------------------------------------------");

  let ld_library_path_value = environment::fetch_multiple_var(
    constants::LD_LIBRARY_PATH_VAR,
    format!("{}/linux64", game_directory()).as_str(),
  );
  info!("Setting up base command");
  info!("Launching With Args: \n{:#?}", &config);
  // Sets the base command for the server
  let mut base_command = command
    .env("SteamAppId", fetch_var("APPID", "892970"))
    .current_dir(game_directory());

  // Sets the name of the server, (Can be set with ENV variable NAME)
  let name = format!("-name {}", fetch_var("NAME", config.name.as_str()));
  base_command.arg(name);

  // Sets the port of the server, (Can be set with ENV variable PORT)
  let port = format!("-port {}", fetch_var("PORT", config.port.as_str()));
  base_command.arg(port);

  // Sets the world of the server, (Can be set with ENV variable WORLD)
  let world = format!("-world {}", fetch_var("WORLD", config.world.as_str()));
  base_command.arg(world);

  // Determines if the server is public or not
  let public = format!("-public {}", fetch_var("PUBLIC", config.public.as_str()));
  base_command.arg(public);

  // Sets the save interval in seconds
  if let Some(save_interval) = &config.save_interval {
    base_command.arg(format!("-saveinterval {}", save_interval));
  };

  // Add set_key to the command
  if let Some(set_key) = &config.set_key {
    base_command.arg(format!("-setkey {}", set_key));
  };

  // Add preset to the command
  if let Some(preset) = &config.preset {
    base_command.arg(format!("-preset {}", preset));
  };

  // Add modifiers to the command
  if let Some(modifiers) = &config.modifiers {
    base_command.args(
      modifiers
          .iter()
          .map(|modifier| format!("-modifier {} {}", modifier.name, modifier.value)),
    );
  };

  // Extra args for the server
  let extra_args = format!(
    "-nographics -batchmode {}",
    fetch_var("SERVER_EXTRA_LAUNCH_ARGS", "")
  )
      .trim()
      .to_string();
  base_command.arg(extra_args);

  let is_public = config.public.eq("1");
  let is_vanilla = fetch_var("TYPE", "vanilla").eq_ignore_ascii_case("vanilla");
  let no_password = config.password.is_empty();

  // If no password env variable
  if !is_public && !is_vanilla && no_password {
    info!("No password found, skipping password flag.")
  } else if no_password && (is_public || is_vanilla) {
    error!("Cannot run you server with no password! PUBLIC must be 0 and cannot be a Vanilla type server.");
    exit(1)
  } else {
    info!("Password found, adding password flag.");
    base_command = base_command.arg(format!("-password {}", config.password));
  }

  if fetch_var("ENABLE_CROSSPLAY", "0").eq("1") {
    info!("Launching with Crossplay! <3");
    base_command = base_command.arg("-crossplay")
  } else {
    info!("No Crossplay Enabled!")
  }

  // Tack on save dir at the end.
  base_command = base_command.arg(format!("-savedir {}", &saves_directory()));

  debug!("Base Command: {:#?}", base_command);

  debug!("Executable: {}", &config.command);
  info!("Launching Command...");
  let bepinex_env = BepInExEnvironment::new();
  if bepinex_env.is_installed() {
    info!("BepInEx detected! Switching to run with BepInEx...");
    info!("BepInEx Environment: \n{:#?}", bepinex_env);
    bepinex_env.launch(base_command)
  } else {
    info!("Everything looks good! Running normally!");
    base_command
      .env(constants::LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .spawn()
  }
}

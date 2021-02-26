pub mod bepinex;

use daemonize::{Daemonize, DaemonizeError};
use log::{debug, error, info};
use sysinfo::{ProcessExt, Signal, System, SystemExt};

use std::{
  io,
  path::Path,
  process::{Child, ExitStatus, Stdio},
  thread,
  time::Duration,
};

use crate::{
  constants,
  executable::{create_execution, execute_mut},
  files::{create_file, ValheimArguments},
  messages,
  steamcmd::steamcmd_command,
  utils::{fetch_env, get_working_dir},
};

pub fn is_installed() -> bool {
  Path::new(&get_working_dir())
    .join(constants::VALHEIM_EXECUTABLE_NAME)
    .exists()
}

fn exit_action() {
  if bepinex::is_bepinex_installed() {
    info!("Server has been started with BepInEx! Keep in mind this may cause errors!!");
    messages::modding_disclaimer()
  }
  info!("Server has been started and Daemonized. It should be online shortly!");
  info!("Keep an eye out for 'Game server connected' in the log!");
  info!("(this indicates its online without any errors.)")
}

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
    .exit_action(exit_action)
    .privileged_action(move || start(&config))
    .start()
}

pub fn start(config: &ValheimArguments) -> io::Result<Child> {
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

pub fn install(app_id: i64) -> io::Result<ExitStatus> {
  info!("Installing {} to {}", app_id, get_working_dir());

  let login = "+login anonymous".to_string();
  let force_install_dir = format!("+force_install_dir {}", get_working_dir());
  let app_update = format!("+app_update {}", app_id);
  let mut steamcmd = steamcmd_command();
  let install_command = steamcmd
    .args(&[login, force_install_dir, app_update])
    .arg("+quit")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
  debug!("Launching install command: {:#?}", install_command);

  execute_mut(install_command)
}

pub fn is_running() -> bool {
  let mut system = System::new();
  system.refresh_processes();
  let valheim_processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);

  !valheim_processes.is_empty()
}

pub fn send_shutdown() {
  info!("Scanning for Valheim process");
  let mut system = System::new();
  system.refresh_all();
  let processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);
  if processes.is_empty() {
    info!("Process NOT found!")
  } else {
    for found_process in processes {
      info!(
        "Found Process with pid {}! Sending Interrupt!",
        found_process.pid()
      );
      if found_process.kill(Signal::Interrupt) {
        info!("Process signal interrupt sent successfully!")
      } else {
        error!("Failed to send signal interrupt!")
      }
    }
  }
}

pub fn wait_for_exit() {
  info!("Waiting for server to completely shutdown...");
  let mut system = System::new();
  loop {
    system.refresh_all();
    let processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);
    if processes.is_empty() {
      break;
    } else {
      // Delay to keep down CPU usage
      thread::sleep(Duration::from_secs(1));
    }
  }
  info!("Server has been shutdown successfully!")
}

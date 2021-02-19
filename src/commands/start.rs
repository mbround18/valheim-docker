use crate::executable::create_execution;
use crate::files::config::{config_file, read_config};
use crate::files::create_file;
use crate::utils::get_working_dir;
use clap::ArgMatches;
use daemonize::Daemonize;
use log::{error, info};
use std::env;
use std::path::Path;
use std::process::{exit, Child, Command};

const LD_LIBRARY_PATH_VAR: &str = "LD_LIBRARY_PATH";
const LD_PRELOAD_VAR: &str = "LD_PRELOAD";
const DYLD_LIBRARY_PATH_VAR: &str = "DYLD_LIBRARY_PATH";
const DYLD_INSERT_LIBRARIES_VAR: &str = "DYLD_INSERT_LIBRARIES";

fn fetch_env(name: &str) -> String {
  match env::var(name) {
    Ok(val) => format!(":{}", val),
    Err(_) => format!(""),
  }
}
fn append_env(name: &str, value: &str) -> String {
  let env_value = fetch_env(name);
  format!("{}{}", value, env_value)
}

fn launch_with_bepinex(command: &mut Command) -> std::io::Result<Child> {
  let doorstop_lib = "libdoorstop_x86.so";
  let doorstop_libs = format!(
    "{working_dir}/doorstop_libs",
    working_dir = get_working_dir()
  );
  let doorstop_insert_lib = format!("{}/{}", &doorstop_libs, doorstop_lib);
  if Path::new(doorstop_insert_lib.as_str()).exists() && Path::new(doorstop_libs.as_str()).exists()
  {
    info!("##########################################################################################################################");
    info!("DISCLAIMER! Modding your server can cause a lot of errors.");
    info!("Please do NOT post issue on the valheim-docker repo based on mod issues.");
    info!("By installing mods, you agree that you will do a root cause analysis to why your server is failing before you make a post.");
    info!("Modding is currently unsupported by the Valheim developers and limited support by the valheim-docker repo.");
    info!("If you have issues please contact the MOD developer FIRST based on the output logs.");
    info!("##########################################################################################################################");
    info!("BepInEx found! Setting up Environment...");
    let ld_preload_value = append_env(LD_PRELOAD_VAR, doorstop_lib);
    let ld_library_path_value = append_env(
      LD_LIBRARY_PATH_VAR,
      format!("{}:{}/linux64", &doorstop_libs, get_working_dir()).as_str(),
    );
    info!("LD_LIBRARY_PATH = {}", ld_library_path_value);
    command
      .env("DOORSTOP_ENABLE", "TRUE")
      .env(
        "DOORSTOP_INVOKE_DLL_PATH",
        format!("{}/BepInEx/core/BepInEx.Preloader.dll", get_working_dir()),
      )
      .env(LD_LIBRARY_PATH_VAR, ld_library_path_value)
      .env(LD_PRELOAD_VAR, ld_preload_value)
      .env(
        DYLD_LIBRARY_PATH_VAR,
        append_env(DYLD_LIBRARY_PATH_VAR, &doorstop_libs),
      )
      .env(
        DYLD_INSERT_LIBRARIES_VAR,
        append_env(DYLD_INSERT_LIBRARIES_VAR, doorstop_insert_lib.as_str()),
      )
      .spawn()
  } else {
    command.spawn()
  }
}

pub fn invoke(args: &ArgMatches) {
  info!("Setting up start scripts...");

  let config = config_file();
  let config_content = read_config(config);
  if config_content.password.len() < 5 {
    error!("The supplied password is too short! It much be 5 characters or greater!");
    exit(1)
  }
  let dry_run: bool = args.is_present("dry_run");
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
      .exit_action(|| info!("Server has been started and Daemonized. It should be online shortly!"))
      .privileged_action(move || {
        let mut command = create_execution(&config_content.command.as_str());
        let ld_library_path_value = append_env(
          LD_LIBRARY_PATH_VAR,
          format!("{}/linux64", get_working_dir()).as_str(),
        );
        command
          .args(&[
            "-nographics",
            "-batchmode",
            "-port",
            &config_content.port.as_str(),
            "-name",
            &config_content.name.as_str(),
            "-world",
            &config_content.world.as_str(),
            "-password",
            &config_content.password.as_str(),
            "-public",
            &config_content.public.as_str(),
          ])
          .env(LD_LIBRARY_PATH_VAR, ld_library_path_value);
        launch_with_bepinex(&mut command)
      });

    match daemonize.start() {
      Ok(_) => info!("Success, daemonized"),
      Err(e) => error!("Error, {}", e),
    }
  } else {
    info!(
      "This command would have launched\n{} -port {} -name {} -world {} -password {} -public {}",
      &config_content.command,
      &config_content.port,
      &config_content.name,
      &config_content.world,
      &config_content.password,
      &config_content.public,
    )
  }
}

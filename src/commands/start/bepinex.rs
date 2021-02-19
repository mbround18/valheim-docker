use crate::commands::start::{append_env, LD_LIBRARY_PATH_VAR, LD_PRELOAD_VAR};
use crate::utils::get_working_dir;
use log::info;
use std::path::Path;
use std::process::{Child, Command};

const DYLD_LIBRARY_PATH_VAR: &str = "DYLD_LIBRARY_PATH";
const DYLD_INSERT_LIBRARIES_VAR: &str = "DYLD_INSERT_LIBRARIES";
const DOORSTOP_ENABLE_VAR: &str = "TRUE";
const DOORSTOP_LIB: &str = "libdoorstop_x86.so";
const DOORSTOP_INVOKE_DLL_PATH_VAR: &str = "DOORSTOP_INVOKE_DLL_PATH";

fn doorstop_libs() -> String {
  format!(
    "{working_dir}/doorstop_libs",
    working_dir = get_working_dir()
  )
}
fn doorstop_insert_lib() -> String {
  format!("{}/{}", doorstop_libs(), DOORSTOP_LIB)
}
pub fn is_bepinex_installed() -> bool {
  Path::new(doorstop_insert_lib().as_str()).exists() && Path::new(doorstop_libs().as_str()).exists()
}

pub fn invoke(command: &mut Command) -> std::io::Result<Child> {
  // ######################################
  info!("##########################################################################################################################");
  info!("DISCLAIMER! Modding your server can cause a lot of errors.");
  info!("Please do NOT post issue on the valheim-docker repo based on mod issues.");
  info!("By installing mods, you agree that you will do a root cause analysis to why your server is failing before you make a post.");
  info!("Modding is currently unsupported by the Valheim developers and limited support by the valheim-docker repo.");
  info!("If you have issues please contact the MOD developer FIRST based on the output logs.");
  info!("##########################################################################################################################");
  // ######################################

  info!("BepInEx found! Setting up Environment...");
  let ld_preload_value = append_env(LD_PRELOAD_VAR, DOORSTOP_LIB);
  let ld_library_path_value = append_env(
    LD_LIBRARY_PATH_VAR,
    format!("{}:{}/linux64", doorstop_libs(), get_working_dir()).as_str(),
  );
  info!("LD_LIBRARY_PATH = {}", ld_library_path_value);
  command
    .env(DOORSTOP_ENABLE_VAR, "TRUE")
    .env(
      DOORSTOP_INVOKE_DLL_PATH_VAR,
      format!("{}/BepInEx/core/BepInEx.Preloader.dll", get_working_dir()),
    )
    .env(LD_LIBRARY_PATH_VAR, ld_library_path_value)
    .env(LD_PRELOAD_VAR, ld_preload_value)
    .env(
      DYLD_LIBRARY_PATH_VAR,
      append_env(DYLD_LIBRARY_PATH_VAR, &*doorstop_libs()),
    )
    .env(
      DYLD_INSERT_LIBRARIES_VAR,
      append_env(DYLD_INSERT_LIBRARIES_VAR, &*doorstop_insert_lib()),
    )
    .spawn()
}

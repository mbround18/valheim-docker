use crate::commands::start::{LD_LIBRARY_PATH_VAR, LD_PRELOAD_VAR};
use crate::utils::{fetch_env, get_working_dir};
use log::{debug, info};
use std::path::Path;
use std::process::{Child, Command};

const DYLD_LIBRARY_PATH_VAR: &str = "DYLD_LIBRARY_PATH";
const DYLD_INSERT_LIBRARIES_VAR: &str = "DYLD_INSERT_LIBRARIES";
const DOORSTOP_ENABLE_VAR: &str = "TRUE";
const DOORSTOP_LIB_VAR: &str = "DOORSTOP_LIB";
const DOORSTOP_LIBS_VAR: &str = "DOORSTOP_LIBS";
const DOORSTOP_INVOKE_DLL_PATH_VAR: &str = "DOORSTOP_INVOKE_DLL_PATH";

fn doorstop_lib() -> String {
  fetch_env(DOORSTOP_LIB_VAR, "libdoorstop_x64.so", false)
}

fn doorstop_libs() -> String {
  fetch_env(
    DOORSTOP_LIBS_VAR,
    format!("{}/doorstop_libs", get_working_dir()).as_str(),
    false,
  )
}

fn doorstop_insert_lib() -> String {
  fetch_env(
    DYLD_INSERT_LIBRARIES_VAR,
    format!("{}/{}", doorstop_libs(), doorstop_lib()).as_str(),
    false,
  )
}

fn doorstop_invoke_dll() -> String {
  fetch_env(
    DOORSTOP_INVOKE_DLL_PATH_VAR,
    format!("{}/BepInEx/core/BepInEx.Preloader.dll", get_working_dir()).as_str(),
    false,
  )
}

pub fn is_bepinex_installed() -> bool {
  let doorstep_insert_lib_exists = Path::new(doorstop_insert_lib().as_str()).exists();
  let doorstep_libs_dir_exists = Path::new(doorstop_libs().as_str()).exists();
  debug!("doorstep insert lib exists: {}", doorstep_insert_lib_exists);
  debug!(
    "doorstep libs directory exists: {}",
    doorstep_libs_dir_exists
  );
  doorstep_insert_lib_exists && doorstep_libs_dir_exists
}

pub struct BepInExEnvironment {
  ld_preload: String,
  ld_library_path: String,
  doorstop_enable: bool,
  doorstop_invoke_dll: String,
  dyld_library_path: String,
  dyld_insert_libraries: String,
}

pub fn build_environment() -> BepInExEnvironment {
  let ld_preload = fetch_env(
    LD_PRELOAD_VAR,
    format!("steamclient.so:{}", doorstop_lib()).as_str(),
    true,
  );
  let ld_library_path = fetch_env(
    LD_LIBRARY_PATH_VAR,
    format!("{}/linux64:{}", get_working_dir(), doorstop_libs()).as_str(),
    true,
  );
  info!("Loading BepInEx Environment...");
  let environment = BepInExEnvironment {
    ld_preload,
    ld_library_path,
    doorstop_enable: true,
    doorstop_invoke_dll: doorstop_invoke_dll(),
    dyld_library_path: fetch_env(DYLD_LIBRARY_PATH_VAR, doorstop_libs().as_str(), true),
    dyld_insert_libraries: fetch_env(
      DYLD_INSERT_LIBRARIES_VAR,
      doorstop_insert_lib().as_str(),
      true,
    ),
  };
  debug!("LD_PRELOAD: {}", &environment.ld_preload);
  debug!("LD_LIBRARY_PATH: {}", &environment.ld_library_path);
  debug!("DOORSTOP_ENABLE: {}", &environment.doorstop_enable);
  debug!("DOORSTOP_INVOKE_DLL: {}", &environment.doorstop_invoke_dll);
  debug!("DYLD_LIBRARY_PATH: {}", &environment.dyld_library_path);
  debug!(
    "DYLD_INSERT_LIBRARIES: {}",
    &environment.dyld_insert_libraries
  );
  environment
}

pub fn invoke(command: &mut Command, environment: &BepInExEnvironment) -> std::io::Result<Child> {
  info!("BepInEx found! Setting up Environment...");
  command
    .env(
      DOORSTOP_ENABLE_VAR,
      &environment.doorstop_enable.to_string().to_uppercase(),
    )
    .env(
      DOORSTOP_INVOKE_DLL_PATH_VAR,
      &environment.doorstop_invoke_dll,
    )
    .env(LD_PRELOAD_VAR, &environment.ld_preload)
    .env(DYLD_LIBRARY_PATH_VAR, &environment.dyld_library_path)
    .env(
      DYLD_INSERT_LIBRARIES_VAR,
      &environment.dyld_insert_libraries,
    )
    .env(LD_LIBRARY_PATH_VAR, &environment.ld_library_path)
    .spawn()
}

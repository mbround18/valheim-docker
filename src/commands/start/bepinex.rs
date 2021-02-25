use crate::commands::start::{LD_LIBRARY_PATH_VAR, LD_PRELOAD_VAR};
use crate::utils::{fetch_env, get_working_dir, path_exists};
use log::{debug, info};
use std::ops::Add;
use std::process::{Child, Command};

const DYLD_LIBRARY_PATH_VAR: &str = "DYLD_LIBRARY_PATH";
const DYLD_INSERT_LIBRARIES_VAR: &str = "DYLD_INSERT_LIBRARIES";
const DOORSTOP_ENABLE_VAR: &str = "DOORSTOP_ENABLE";
const DOORSTOP_LIB_VAR: &str = "DOORSTOP_LIB";
const DOORSTOP_LIBS_VAR: &str = "DOORSTOP_LIBS";
const DOORSTOP_INVOKE_DLL_PATH_VAR: &str = "DOORSTOP_INVOKE_DLL_PATH";
const DOORSTOP_CORLIB_OVERRIDE_PATH_VAR: &str = "DOORSTOP_CORLIB_OVERRIDE_PATH";

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
  let default = format!("{}/{}", doorstop_libs(), doorstop_lib().replace(":", ""));
  fetch_env(DYLD_INSERT_LIBRARIES_VAR, default.as_str(), false)
}

fn doorstop_invoke_dll() -> String {
  fetch_env(
    DOORSTOP_INVOKE_DLL_PATH_VAR,
    format!("{}/BepInEx/core/BepInEx.Preloader.dll", get_working_dir()).as_str(),
    false,
  )
}

pub struct BepInExEnvironment {
  ld_preload: String,
  ld_library_path: String,
  doorstop_enable: String,
  doorstop_invoke_dll: String,
  doorstop_corlib_override_path: String,
  dyld_library_path: String,
  dyld_insert_libraries: String,
}

pub fn build_environment() -> BepInExEnvironment {
  let ld_preload = fetch_env(LD_PRELOAD_VAR, "", false).add(doorstop_lib().as_str());
  let ld_library_path = fetch_env(
    LD_LIBRARY_PATH_VAR,
    format!("./linux64:{}", doorstop_libs()).as_str(),
    false,
  );
  let doorstop_invoke_dll_value = doorstop_invoke_dll();
  let doorstop_corlib_override_path = fetch_env(
    DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
    format!("{}/{}", get_working_dir(), "unstripped_corlib").as_str(),
    false,
  );
  let dyld_library_path = fetch_env(DYLD_LIBRARY_PATH_VAR, doorstop_libs().as_str(), false);
  let dyld_insert_libraries = fetch_env(
    DYLD_INSERT_LIBRARIES_VAR,
    doorstop_insert_lib().as_str(),
    false,
  );
  info!("Checking for BepInEx Environment...");
  let environment = BepInExEnvironment {
    ld_preload,
    ld_library_path,
    doorstop_enable: true.to_string().to_uppercase(),
    doorstop_invoke_dll: doorstop_invoke_dll_value,
    doorstop_corlib_override_path,
    dyld_library_path,
    dyld_insert_libraries,
  };
  debug!("LD_PRELOAD: {}", &environment.ld_preload);
  debug!("LD_LIBRARY_PATH: {}", &environment.ld_library_path);
  debug!("DOORSTOP_ENABLE: {}", &environment.doorstop_enable);
  debug!(
    "DOORSTOP_INVOKE_DLL_PATH: {}",
    &environment.doorstop_invoke_dll
  );
  debug!(
    "{}: {}",
    DOORSTOP_CORLIB_OVERRIDE_PATH_VAR, &environment.doorstop_corlib_override_path
  );
  debug!("DYLD_LIBRARY_PATH: {}", &environment.dyld_library_path);
  debug!(
    "DYLD_INSERT_LIBRARIES: {}",
    &environment.dyld_insert_libraries
  );
  environment
}

pub fn is_bepinex_installed() -> bool {
  let bepinex_env: BepInExEnvironment = build_environment();
  debug!("Checking for BepInEx specific files...");
  let checks = &[
    bepinex_env.doorstop_corlib_override_path,
    bepinex_env.dyld_insert_libraries,
    bepinex_env.dyld_library_path,
    bepinex_env.doorstop_invoke_dll,
  ];
  let expected_state = true;
  let output = checks.iter().all(|ref v| path_exists(&v) == expected_state);
  if output {
    debug!("Yay! looks like we found all the required files for BepInEx to run! <3")
  } else {
    debug!("Uhh ohh!!! Looks like you are missing something.")
  }
  output
}

pub fn invoke(command: &mut Command, environment: &BepInExEnvironment) -> std::io::Result<Child> {
  info!("BepInEx found! Setting up Environment...");
  command
    // DOORSTOP_ENABLE must not have quotes around it.
    .env(DOORSTOP_ENABLE_VAR, &environment.doorstop_enable)
    // DOORSTOP_INVOKE_DLL_PATH must not have quotes around it.
    .env(
      DOORSTOP_INVOKE_DLL_PATH_VAR,
      &environment.doorstop_invoke_dll,
    )
    .env(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      &environment.doorstop_corlib_override_path,
    )
    // LD_LIBRARY_PATH must not have quotes around it.
    .env(LD_LIBRARY_PATH_VAR, &environment.ld_library_path)
    // LD_PRELOAD must not have quotes around it.
    .env(LD_PRELOAD_VAR, &environment.ld_preload)
    // DYLD_LIBRARY_PATH is weird af and MUST have quotes around it.
    .env(
      DYLD_LIBRARY_PATH_VAR,
      format!("\"{}\"", &environment.dyld_library_path),
    )
    // DYLD_INSERT_LIBRARIES must not have quotes around it.
    .env(
      DYLD_INSERT_LIBRARIES_VAR,
      &environment.dyld_insert_libraries,
    )
    .spawn()
}

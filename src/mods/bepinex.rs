use crate::constants;
use crate::utils::common_paths::{bepinex_directory, game_directory};
use crate::utils::{environment, path_exists};
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

fn parse_path(env_var: &str, default: String, alt: String) -> String {
  let output = environment::fetch_var(env_var, &default);
  if !path_exists(&output) && path_exists(&alt) {
    String::from(&alt)
  } else {
    String::from(&output)
  }
}

#[derive(Debug)]
pub struct BepInExEnvironment {
  ld_preload: String,
  ld_library_path: String,
  doorstop_enable: String,
  doorstop_invoke_dll: String,
  doorstop_corlib_override_path: String,
  dyld_library_path: String,
  dyld_insert_libraries: String,
}

impl BepInExEnvironment {
  pub fn new() -> BepInExEnvironment {
    let game_dir = game_directory();
    let bepinex_dir = bepinex_directory();
    let bepinex_preloader_dll = format!("{}/core/BepInEx.Preloader.dll", &bepinex_dir);

    debug!("Parsing Doorstop locations.");
    let doorstop_lib = environment::fetch_var(DOORSTOP_LIB_VAR, "libdoorstop_x64.so");
    let doorstop_libs = parse_path(
      DOORSTOP_LIBS_VAR,
      format!("{}/doorstop_libs", &game_dir),
      format!("{}/doorstop", &bepinex_dir),
    );
    let doorstop_invoke_dll =
      environment::fetch_var(DOORSTOP_INVOKE_DLL_PATH_VAR, &bepinex_preloader_dll);
    let doorstop_corlib_override_path = parse_path(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      format!("{}/unstripped_corlib", &game_dir),
      format!("{}/core_lib", &bepinex_dir),
    );
    let doorstop_base_dll = format!("{}/{}", &doorstop_libs, &doorstop_lib);

    debug!("Parsing LD locations.");
    let ld_preload = environment::fetch_var(constants::LD_PRELOAD_VAR, "").add(&doorstop_lib);
    let ld_library_path = environment::fetch_var(
      constants::LD_LIBRARY_PATH_VAR,
      format!("./linux64:{}", &doorstop_libs).as_str(),
    );

    debug!("Parsing LD locations.");
    let dyld_library_path = environment::fetch_var(DYLD_LIBRARY_PATH_VAR, &doorstop_libs);
    let dyld_insert_libraries =
      environment::fetch_var(DYLD_INSERT_LIBRARIES_VAR, &doorstop_base_dll);

    debug!("Returning environment");
    BepInExEnvironment {
      ld_preload,
      ld_library_path,
      doorstop_enable: true.to_string().to_uppercase(),
      doorstop_invoke_dll,
      doorstop_corlib_override_path,
      dyld_library_path,
      dyld_insert_libraries,
    }
  }
  pub fn is_installed(&self) -> bool {
    debug!("Checking for BepInEx specific files...");
    let checks = &[
      &self.doorstop_corlib_override_path,
      &self.dyld_insert_libraries,
      &self.dyld_library_path,
      &self.doorstop_invoke_dll,
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

  pub fn launch(&self, command: &mut Command) -> std::io::Result<Child> {
    info!("BepInEx found! Setting up Environment...");
    command
      // DOORSTOP_ENABLE must not have quotes around it.
      .env(DOORSTOP_ENABLE_VAR, &self.doorstop_enable)
      // DOORSTOP_INVOKE_DLL_PATH must not have quotes around it.
      .env(DOORSTOP_INVOKE_DLL_PATH_VAR, &self.doorstop_invoke_dll)
      .env(
        DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
        &self.doorstop_corlib_override_path,
      )
      // LD_LIBRARY_PATH must not have quotes around it.
      .env(constants::LD_LIBRARY_PATH_VAR, &self.ld_library_path)
      // LD_PRELOAD must not have quotes around it.
      .env(constants::LD_PRELOAD_VAR, &self.ld_preload)
      // DYLD_LIBRARY_PATH is weird af and MUST have quotes around it.
      .env(
        DYLD_LIBRARY_PATH_VAR,
        format!("\"{}\"", &self.dyld_library_path),
      )
      // DYLD_INSERT_LIBRARIES must not have quotes around it.
      .env(DYLD_INSERT_LIBRARIES_VAR, &self.dyld_insert_libraries)
      .spawn()
  }
}

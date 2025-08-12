use std::ops::Add;
use std::process::{Child, Command};

use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::constants;
use crate::mods::manifest::Manifest;
use crate::utils::common_paths::{bepinex_directory, bepinex_plugin_directory, game_directory};
use crate::utils::{environment, path_exists};
use semver::Version;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

const DOORSTOP_LIB_VAR: &str = "DOORSTOP_LIB";
const DOORSTOP_LIBS_VAR: &str = "DOORSTOP_LIBS";
// Doorstop 4.x and above
const DOORSTOP_ENABLED_VAR: &str = "DOORSTOP_ENABLED";
const DOORSTOP_TARGET_ASSEMBLY_VAR: &str = "DOORSTOP_TARGET_ASSEMBLY";
// Compatibility with older Doorstop versions
const DOORSTOP_ENABLE_VAR: &str = "DOORSTOP_ENABLE";
const DOORSTOP_INVOKE_DLL_PATH_VAR: &str = "DOORSTOP_INVOKE_DLL_PATH";
const DOORSTOP_CORLIB_OVERRIDE_PATH_VAR: &str = "DOORSTOP_CORLIB_OVERRIDE_PATH";

// Minimum BepInExPack_Valheim version that uses Doorstop 4.x+ semantics
const DOORSTOP_V4_MIN_VERSION_STR: &str = "5.4.2330";
static DOORSTOP_V4_MIN_VERSION: OnceLock<Version> = OnceLock::new();

fn doorstop_v4_min() -> &'static Version {
  DOORSTOP_V4_MIN_VERSION
    .get_or_init(|| Version::parse(DOORSTOP_V4_MIN_VERSION_STR).expect("valid semver"))
}

fn is_v4_or_newer(version_str: &str) -> Option<bool> {
  Version::parse(version_str)
    .ok()
    .map(|v| v >= *doorstop_v4_min())
}

fn detect_doorstop_mode_from_manifest(manifest_path: &Path) -> bool {
  if let Ok(manifest) = Manifest::try_from(manifest_path.to_path_buf()) {
    if let Some(ver) = manifest.version_number.as_ref() {
      return is_v4_or_newer(ver).unwrap_or(false);
    }
  }
  false
}

fn parse_path(env_var: &str, default: String, alt: String) -> String {
  let output = environment::fetch_var(env_var, &default);
  if !path_exists(&output) && path_exists(&alt) {
    String::from(&alt)
  } else {
    String::from(&output)
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModInfo {
  pub(crate) name: String,
  location: String,
}

#[derive(Debug)]
pub struct BepInExEnvironment {
  doorstop_lib: String,
  ld_preload: String,
  ld_library_path: String,
  // Doorstop 4.x and above
  doorstop_enabled: String,
  doorstop_target_assembly: String,
  // Compatibility with older Doorstop versions
  doorstop_enable: String,
  doorstop_invoke_dll: String,
  doorstop_corlib_override_path: String,
  doorstop_is_v4_plus: bool,
}
impl Default for BepInExEnvironment {
  fn default() -> Self {
    Self::new()
  }
}

impl BepInExEnvironment {
  /// Test-friendly constructor to avoid mutating global environment.
  #[cfg(test)]
  pub fn from_game_dir<P: AsRef<Path>>(game_dir_path: P) -> BepInExEnvironment {
    let game_dir = game_dir_path.as_ref().to_string_lossy().to_string();
    let bepinex_dir = format!("{}/BepInEx", &game_dir);
    let bepinex_preloader_dll = format!("{}/core/BepInEx.Preloader.dll", &bepinex_dir);

    // Detect mode from the manifest inside provided game_dir
    let doorstop_is_v4_plus =
      detect_doorstop_mode_from_manifest(&PathBuf::from(&bepinex_dir).join("manifest.json"));

    debug!("Parsing Doorstop locations.");
    let doorstop_lib = environment::fetch_var(
      DOORSTOP_LIB_VAR,
      &format!("{}/libdoorstop_x64.so", &game_dir),
    );
    let doorstop_libs = parse_path(
      DOORSTOP_LIBS_VAR,
      format!("{}/doorstop_libs", &game_dir),
      format!("{}/doorstop", &bepinex_dir),
    );
    let doorstop_target_assembly =
      environment::fetch_var(DOORSTOP_TARGET_ASSEMBLY_VAR, &bepinex_preloader_dll);
    let doorstop_invoke_dll =
      environment::fetch_var(DOORSTOP_INVOKE_DLL_PATH_VAR, &bepinex_preloader_dll);
    let doorstop_corlib_override_path = parse_path(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      format!("{}/unstripped_corlib", &game_dir),
      format!("{}/core_lib", &bepinex_dir),
    );

    debug!("Parsing LD locations.");
    let ld_preload = environment::fetch_var(constants::LD_PRELOAD_VAR, "").add(&doorstop_lib);
    let ld_library_path = environment::fetch_var(
      constants::LD_LIBRARY_PATH_VAR,
      format!("./linux64:{}", &doorstop_libs).as_str(),
    );

    debug!("Returning environment");
    BepInExEnvironment {
      doorstop_lib,
      ld_preload,
      ld_library_path,
      // Doorstop 4.x and above
      doorstop_enabled: 1.to_string(),
      doorstop_target_assembly,
      // Compatibility with older Doorstop versions
      doorstop_enable: true.to_string().to_uppercase(),
      doorstop_invoke_dll,
      doorstop_corlib_override_path,
      doorstop_is_v4_plus,
    }
  }
  pub fn new() -> BepInExEnvironment {
    let game_dir = game_directory();
    let bepinex_dir = bepinex_directory();
    let bepinex_preloader_dll = format!("{}/core/BepInEx.Preloader.dll", &bepinex_dir);

    // Detect BepInExPack_Valheim version from manifest.json if present
    let doorstop_is_v4_plus =
      detect_doorstop_mode_from_manifest(&PathBuf::from(&bepinex_dir).join("manifest.json"));

    debug!("Parsing Doorstop locations.");
    let doorstop_lib = environment::fetch_var(DOORSTOP_LIB_VAR, "libdoorstop_x64.so");
    let doorstop_libs = parse_path(
      DOORSTOP_LIBS_VAR,
      format!("{}/doorstop_libs", &game_dir),
      format!("{}/doorstop", &bepinex_dir),
    );
    let doorstop_target_assembly =
      environment::fetch_var(DOORSTOP_TARGET_ASSEMBLY_VAR, &bepinex_preloader_dll);
    let doorstop_invoke_dll =
      environment::fetch_var(DOORSTOP_INVOKE_DLL_PATH_VAR, &bepinex_preloader_dll);
    let doorstop_corlib_override_path = parse_path(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      format!("{}/unstripped_corlib", &game_dir),
      format!("{}/core_lib", &bepinex_dir),
    );

    debug!("Parsing LD locations.");
    let ld_preload = environment::fetch_var(constants::LD_PRELOAD_VAR, "").add(&doorstop_lib);
    let ld_library_path = environment::fetch_var(
      constants::LD_LIBRARY_PATH_VAR,
      format!("./linux64:{}", &doorstop_libs).as_str(),
    );

    debug!("Returning environment");
    BepInExEnvironment {
      doorstop_lib,
      ld_preload,
      ld_library_path,
      // Doorstop 4.x and above
      doorstop_enabled: 1.to_string(),
      doorstop_target_assembly,
      // Compatibility with older Doorstop versions
      doorstop_enable: true.to_string().to_uppercase(),
      doorstop_invoke_dll,
      doorstop_corlib_override_path,
      doorstop_is_v4_plus,
    }
  }
  pub fn is_installed(&self) -> bool {
    debug!("Checking for BepInEx specific files...");
    // Choose checks based on detected mode
    let checks_v4 = [&self.doorstop_lib, &self.doorstop_target_assembly];
    let checks_v3 = [&self.doorstop_lib, &self.doorstop_invoke_dll];
    let expected_state = true;
    let output = if self.doorstop_is_v4_plus {
      checks_v4.iter().all(|v| path_exists(v) == expected_state)
    } else {
      checks_v3.iter().all(|v| path_exists(v) == expected_state)
    };
    if output {
      debug!("Yay! looks like we found all the required files for BepInEx to run! <3")
    } else {
      debug!("We didn't find a modded instance! Launching a normal instance!")
    }
    output
  }

  pub fn list_mods(&self) -> Vec<ModInfo> {
    if self.is_installed() {
      glob::glob(&format!("{}/**/*.dll", bepinex_plugin_directory()))
        .unwrap()
        .map(|file| {
          let found_file = file.unwrap();
          let location = found_file.as_path().to_str().unwrap().to_string();
          let name = found_file
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
          ModInfo { name, location }
        })
        .collect()
    } else {
      vec![]
    }
  }

  pub fn launch(&self, mut command: Command) -> std::io::Result<Child> {
    info!("BepInEx found! Setting up Environment...");
    // The env variables must not have quotes around them
    command
      .env(constants::LD_LIBRARY_PATH_VAR, &self.ld_library_path)
      .env(constants::LD_PRELOAD_VAR, &self.ld_preload);
    if self.doorstop_is_v4_plus {
      command
        .env(DOORSTOP_ENABLED_VAR, &self.doorstop_enabled)
        .env(DOORSTOP_TARGET_ASSEMBLY_VAR, &self.doorstop_target_assembly)
    } else {
      command
        .env(DOORSTOP_ENABLE_VAR, &self.doorstop_enable)
        .env(DOORSTOP_INVOKE_DLL_PATH_VAR, &self.doorstop_invoke_dll)
        .env(
          DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
          &self.doorstop_corlib_override_path,
        )
    };
    command.spawn()
  }
}

#[cfg(test)]
mod bepinex_tests {
  use super::*;
  use serial_test::serial;
  use std::fs;
  use std::fs::File;
  use std::io::Write as _;
  use tempfile::tempdir;

  struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
  }

  impl EnvGuard {
    fn capture(keys: &[&str]) -> Self {
      let saved = keys
        .iter()
        .map(|k| (k.to_string(), std::env::var(k).ok()))
        .collect();
      EnvGuard { saved }
    }
  }

  impl Drop for EnvGuard {
    fn drop(&mut self) {
      for (k, v) in self.saved.drain(..) {
        match v {
          Some(val) => std::env::set_var(&k, val),
          None => std::env::remove_var(&k),
        }
      }
    }
  }

  fn write_file(path: &Path) {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).unwrap();
    }
    File::create(path).unwrap();
  }

  fn write_text(path: &Path, s: &str) {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).unwrap();
    }
    let mut f = File::create(path).unwrap();
    f.write_all(s.as_bytes()).unwrap();
  }

  #[test]
  fn version_gate_old_is_legacy() {
    assert_eq!(is_v4_or_newer("5.4.2202"), Some(false));
  }

  #[test]
  fn version_gate_threshold_is_v4() {
    assert_eq!(is_v4_or_newer(DOORSTOP_V4_MIN_VERSION_STR), Some(true));
  }

  #[test]
  fn version_gate_newer_is_v4() {
    assert_eq!(is_v4_or_newer("5.4.3000"), Some(true));
  }

  #[test]
  fn version_gate_invalid_is_none() {
    assert_eq!(is_v4_or_newer("invalid"), None);
  }

  #[test]
  #[serial]
  fn detect_mode_from_manifest_v4_true() {
    let _guard = EnvGuard::capture(&[]);
    let tmp = tempdir().unwrap();
    let game = tmp.path();
    // Create BepInEx manifest with v4 threshold
    let bepinex = game.join("BepInEx");
    write_text(
      &bepinex.join("manifest.json"),
      &format!(
        "{{\n  \"name\": \"BepInExPack_Valheim\",\n  \"version_number\": \"{}\"\n}}",
        DOORSTOP_V4_MIN_VERSION_STR
      ),
    );

    // Create required files for checks
    let doorstop_so = game.join("libdoorstop_x64.so");
    write_file(&doorstop_so);
    write_file(&bepinex.join("core/BepInEx.Preloader.dll"));

    let env = BepInExEnvironment::from_game_dir(game);
    assert!(env.doorstop_is_v4_plus);
    assert!(env.is_installed());
  }

  #[test]
  #[serial]
  fn detect_mode_from_manifest_v3_true() {
    let _guard = EnvGuard::capture(&[]);
    let tmp = tempdir().unwrap();
    let game = tmp.path();
    let bepinex = game.join("BepInEx");
    write_text(
      &bepinex.join("manifest.json"),
      "{\n  \"name\": \"BepInExPack_Valheim\",\n  \"version_number\": \"5.4.2202\"\n}",
    );
    let doorstop_so = game.join("libdoorstop_x64.so");
    write_file(&doorstop_so);
    // For v3, invoke dll path exists (same preloader dll)
    write_file(&bepinex.join("core/BepInEx.Preloader.dll"));

    let env = BepInExEnvironment::from_game_dir(game);
    assert!(!env.doorstop_is_v4_plus);
    assert!(env.is_installed());
  }

  #[test]
  #[serial]
  fn list_mods_discovers_plugins() {
    let _guard = EnvGuard::capture(&[crate::constants::GAME_LOCATION, DOORSTOP_LIB_VAR]);
    let tmp = tempdir().unwrap();
    let game = tmp.path();
    // Point GAME_LOCATION so bepinex_plugin_directory() resolves into this temp
    std::env::set_var(crate::constants::GAME_LOCATION, game);
    let bepinex = game.join("BepInEx");
    // Ensure v4 mode to satisfy checks
    write_text(
      &bepinex.join("manifest.json"),
      &format!(
        "{{\n  \"name\": \"BepInExPack_Valheim\",\n  \"version_number\": \"{}\"\n}}",
        DOORSTOP_V4_MIN_VERSION_STR
      ),
    );
    let doorstop_so = game.join("libdoorstop_x64.so");
    write_file(&doorstop_so);
    write_file(&bepinex.join("core/BepInEx.Preloader.dll"));

    // Create plugin dll
    write_file(&bepinex.join("plugins/MyCoolMod/plugin.dll"));

    let env = BepInExEnvironment::from_game_dir(game);
    let mods = env.list_mods();
    assert!(mods.iter().any(|m| m.name == "plugin.dll"));
  }

  #[test]
  #[serial]
  fn parse_path_prefers_alt_when_default_missing() {
    let tmp = tempdir().unwrap();
    let default = tmp.path().join("missing_dir");
    let alt = tmp.path().join("exists_dir");
    fs::create_dir_all(&alt).unwrap();
    // Use a throwaway env var name
    let var = "ODIN_TEST_PARSE_PATH";
    std::env::remove_var(var);
    let out = parse_path(
      var,
      default.to_string_lossy().into(),
      alt.to_string_lossy().into(),
    );
    assert_eq!(Path::new(&out), alt);
  }
}

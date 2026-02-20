use std::ops::Add;
use std::process::{Child, Command};

use log::debug;
use serde::{Deserialize, Serialize};

use crate::constants;
use crate::executable::execute_mut;
use crate::mods::installed_mods::installed_mods_with_paths;
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

fn detect_doorstop_mode_from_fs(game_dir: &str) -> bool {
  // Consider Doorstop v4+ if doorstop_libs exists and contains either .so or .dylib
  let libs = PathBuf::from(game_dir).join("doorstop_libs");
  if !libs.exists() {
    return false;
  }
  let so = libs.join("libdoorstop_x64.so");
  let dylib = libs.join("libdoorstop_64.dylib");
  so.exists() || dylib.exists()
}

fn parse_path(env_var: &str, default: String, alt: String) -> String {
  let output = environment::fetch_var(env_var, &default);
  if !path_exists(&output) && path_exists(&alt) {
    String::from(&alt)
  } else {
    String::from(&output)
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModInfo {
  pub(crate) name: String,
  location: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  version: Option<String>,
}

#[derive(Debug)]
pub struct BepInExEnvironment {
  doorstop_lib: String,
  doorstop_libs_dir: String,
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

    // Detect mode from the manifest and filesystem
    let mut doorstop_is_v4_plus =
      detect_doorstop_mode_from_manifest(&PathBuf::from(&bepinex_dir).join("manifest.json"));
    if !doorstop_is_v4_plus {
      doorstop_is_v4_plus = detect_doorstop_mode_from_fs(&game_dir);
    }

    debug!("Parsing Doorstop locations.");
    let doorstop_lib_default = if doorstop_is_v4_plus {
      format!("{}/doorstop_libs/libdoorstop_x64.so", &game_dir)
    } else {
      format!("{}/libdoorstop_x64.so", &game_dir)
    };
    let doorstop_lib = environment::fetch_var(DOORSTOP_LIB_VAR, &doorstop_lib_default);
    let doorstop_libs = parse_path(
      DOORSTOP_LIBS_VAR,
      format!("{}/doorstop_libs", &game_dir),
      format!("{}/doorstop", &bepinex_dir),
    );
    let doorstop_target_assembly =
      environment::fetch_var(DOORSTOP_TARGET_ASSEMBLY_VAR, &bepinex_preloader_dll);
    let doorstop_invoke_dll =
      environment::fetch_var(DOORSTOP_INVOKE_DLL_PATH_VAR, &bepinex_preloader_dll);
    // Prefer BepInEx/core_lib, fallback to BepInEx/core; no more game_dir/unstripped_corlib
    let doorstop_corlib_override_path = parse_path(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      format!("{}/core_lib", &bepinex_dir),
      format!("{}/core", &bepinex_dir),
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
      doorstop_libs_dir: doorstop_libs.clone(),
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

    // Detect BepInExPack_Valheim version from manifest.json if present and cross-check filesystem
    let mut doorstop_is_v4_plus =
      detect_doorstop_mode_from_manifest(&PathBuf::from(&bepinex_dir).join("manifest.json"));
    if !doorstop_is_v4_plus {
      doorstop_is_v4_plus = detect_doorstop_mode_from_fs(&game_dir);
    }

    debug!("Parsing Doorstop locations.");
    let doorstop_lib_default = if doorstop_is_v4_plus {
      format!("{}/doorstop_libs/libdoorstop_x64.so", &game_dir)
    } else {
      String::from("libdoorstop_x64.so")
    };
    let doorstop_lib = environment::fetch_var(DOORSTOP_LIB_VAR, &doorstop_lib_default);
    let doorstop_libs = parse_path(
      DOORSTOP_LIBS_VAR,
      format!("{}/doorstop_libs", &game_dir),
      format!("{}/doorstop", &bepinex_dir),
    );
    let doorstop_target_assembly =
      environment::fetch_var(DOORSTOP_TARGET_ASSEMBLY_VAR, &bepinex_preloader_dll);
    let doorstop_invoke_dll =
      environment::fetch_var(DOORSTOP_INVOKE_DLL_PATH_VAR, &bepinex_preloader_dll);
    // Prefer BepInEx/core_lib, fallback to BepInEx/core; no more game_dir/unstripped_corlib
    let doorstop_corlib_override_path = parse_path(
      DOORSTOP_CORLIB_OVERRIDE_PATH_VAR,
      format!("{}/core_lib", &bepinex_dir),
      format!("{}/core", &bepinex_dir),
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
      doorstop_libs_dir: doorstop_libs.clone(),
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
    // For Doorstop v4+: verify native lib in doorstop_libs dir (Linux .so or macOS .dylib)
    let lib_so = format!("{}/libdoorstop_x64.so", &self.doorstop_libs_dir);
    let lib_dylib = format!("{}/libdoorstop_64.dylib", &self.doorstop_libs_dir);
    let checks_v4 = [&self.doorstop_target_assembly];
    let checks_v3 = [&self.doorstop_lib, &self.doorstop_invoke_dll];
    let expected_state = true;
    let output = if self.doorstop_is_v4_plus {
      let so_exists = path_exists(&lib_so);
      let dylib_exists = path_exists(&lib_dylib);
      let target_exists = path_exists(&self.doorstop_target_assembly);
      debug!("Doorstop v4+ checks:");
      debug!(" - lib (so): {} => {}", &lib_so, so_exists);
      debug!(" - lib (dylib): {} => {}", &lib_dylib, dylib_exists);
      debug!(
        " - target assembly: {} => {}",
        &self.doorstop_target_assembly, target_exists
      );
      (so_exists || dylib_exists) && checks_v4.iter().all(|v| path_exists(v) == expected_state)
    } else {
      let lib_exists = path_exists(&self.doorstop_lib);
      let invoke_exists = path_exists(&self.doorstop_invoke_dll);
      debug!("Doorstop v3 checks:");
      debug!(" - doorstop_lib: {} => {}", &self.doorstop_lib, lib_exists);
      debug!(
        " - invoke dll: {} => {}",
        &self.doorstop_invoke_dll, invoke_exists
      );
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
    if !self.is_installed() {
      return vec![];
    }

    // Prefer manifests returned by installed_mods_with_paths (driven by MODS_LOCATION)
    let from_manifests: Vec<ModInfo> = installed_mods_with_paths()
      .into_iter()
      .map(|im| ModInfo {
        name: im.manifest.name,
        location: im.path,
        version: im.manifest.version_number,
      })
      .collect();

    if !from_manifests.is_empty() {
      return from_manifests;
    }

    // Fallback to scanning plugin DLLs and trying to infer a nearby manifest.json
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
        let version = find_manifest_for_plugin(found_file.as_path())
          .and_then(|m| Manifest::try_from(m).ok())
          .and_then(|mf| mf.version_number);
        ModInfo {
          name,
          location,
          version,
        }
      })
      .collect()
  }

  pub fn launch(&self, mut command: Command) -> std::io::Result<Child> {
    debug!("BepInEx found! Setting up Environment...");
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
    execute_mut(&mut command)
  }
}

fn find_manifest_for_plugin(dll_path: &Path) -> Option<PathBuf> {
  // Check the directory containing the dll, then its parent, stopping at plugins root
  let mut dir_opt = dll_path.parent();
  for _ in 0..2 {
    if let Some(dir) = dir_opt {
      let candidate = dir.join("manifest.json");
      if candidate.exists() {
        return Some(candidate);
      }
      // Stop if we've reached the plugins root folder
      if dir.file_name().map(|n| n == "plugins").unwrap_or(false) {
        break;
      }
      dir_opt = dir.parent();
    }
  }
  None
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
    // Doorstop v4 expects libraries under doorstop_libs
    let doorstop_libs = game.join("doorstop_libs");
    fs::create_dir_all(&doorstop_libs).unwrap();
    let doorstop_so = doorstop_libs.join("libdoorstop_x64.so");
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
    let doorstop_libs = game.join("doorstop_libs");
    fs::create_dir_all(&doorstop_libs).unwrap();
    let doorstop_so = doorstop_libs.join("libdoorstop_x64.so");
    write_file(&doorstop_so);
    write_file(&bepinex.join("core/BepInEx.Preloader.dll"));

    // Create plugin dll and a manifest with version_number next to it
    let mod_dir = bepinex.join("plugins/MyCoolMod");
    write_file(&mod_dir.join("plugin.dll"));
    write_text(
      &mod_dir.join("manifest.json"),
      "{\n  \"name\": \"MyCoolMod\",\n  \"version_number\": \"1.2.3\"\n}",
    );

    let env = BepInExEnvironment::from_game_dir(game);
    let mods = env.list_mods();
    assert!(mods.iter().any(|m| m.name == "plugin.dll"));
    let plugin = mods.iter().find(|m| m.name == "plugin.dll").unwrap();
    assert_eq!(plugin.version.as_deref(), Some("1.2.3"));
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

  #[test]
  #[serial]
  fn list_mods_uses_installed_mods_when_available() {
    let _guard = EnvGuard::capture(&[
      crate::constants::GAME_LOCATION,
      crate::constants::MODS_LOCATION,
      DOORSTOP_LIB_VAR,
    ]);

    // Prepare a temp game dir with minimal v4 files
    let tmp = tempdir().unwrap();
    let game = tmp.path().join("game");
    let modsroot = tmp.path().join("modsroot");
    fs::create_dir_all(&modsroot).unwrap();
    std::env::set_var(crate::constants::MODS_LOCATION, &modsroot);

    let bepinex = game.join("BepInEx");
    write_text(
      &bepinex.join("manifest.json"),
      &format!(
        "{{\n  \"name\": \"BepInExPack_Valheim\",\n  \"version_number\": \"{}\"\n}}",
        DOORSTOP_V4_MIN_VERSION_STR
      ),
    );
    let doorstop_libs = game.join("doorstop_libs");
    fs::create_dir_all(&doorstop_libs).unwrap();
    write_file(&doorstop_libs.join("libdoorstop_x64.so"));
    write_file(&bepinex.join("core/BepInEx.Preloader.dll"));

    // Create an installed mod manifest under MODS_LOCATION
    let mod_dir = modsroot.join("HelloMod");
    write_text(
      &mod_dir.join("manifest.json"),
      "{\n  \"name\": \"HelloMod\",\n  \"version_number\": \"9.9.9\"\n}",
    );

    let env = BepInExEnvironment::from_game_dir(&game);
    let mods = env.list_mods();
    assert!(mods.iter().any(|m| m.name == "HelloMod"));
    let hello = mods.iter().find(|m| m.name == "HelloMod").unwrap();
    assert_eq!(hello.version.as_deref(), Some("9.9.9"));
    assert!(hello.location.ends_with("manifest.json"));
  }
}

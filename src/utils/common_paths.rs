use crate::constants::{GAME_LOCATION, MODS_LOCATION, SAVE_LOCATION};
use crate::utils::get_working_dir;
use std::env;

pub fn game_directory() -> String {
  env::var(GAME_LOCATION).unwrap_or_else(|_| get_working_dir())
}

pub fn bepinex_directory() -> String {
  format!("{}/BepInEx", game_directory())
}

pub fn bepinex_plugin_directory() -> String {
  format!("{}/plugins", bepinex_directory())
}

pub fn mods_directory() -> String {
  env::var(MODS_LOCATION).unwrap_or_else(|_| format!("{}/mods", get_working_dir()))
}

// pub fn backup_directory() -> String {
//   env::var(BACKUP_LOCATION).unwrap_or_else(|_| format!("{}/backups", get_working_dir()))
// }

pub fn saves_directory() -> String {
  env::var(SAVE_LOCATION).unwrap_or_else(|_| match env::var("HOME") {
    Ok(dir) => format!("{}/.config/unity3d/IronGate/Valheim", dir),
    Err(_) => format!("{}/backups", get_working_dir()),
  })
}

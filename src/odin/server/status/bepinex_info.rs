use crate::mods::bepinex::{BepInExEnvironment, ModInfo};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BepInExInfo {
  pub enabled: bool,
  pub(crate) mods: Vec<ModInfo>,
}

impl BepInExInfo {
  pub(crate) fn new() -> BepInExInfo {
    let env = BepInExEnvironment::new();
    BepInExInfo {
      enabled: env.is_installed(),
      mods: env.list_mods(),
    }
  }
  pub fn disabled() -> BepInExInfo {
    BepInExInfo {
      enabled: false,
      mods: vec![],
    }
  }
}

use odin::installed_mods_with_paths;
use serde::Serialize;
use warp::reply::{json, Json};

#[derive(Serialize)]
#[serde(crate = "serde")]
pub struct ModInfo {
  pub name: String,
  pub version: Option<String>,
  pub dependencies: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(crate = "serde")]
pub struct ModsResponse {
  pub installed_mods: Vec<ModInfo>,
  pub count: usize,
}

pub fn invoke() -> Json {
  let installed = installed_mods_with_paths();

  let mods: Vec<ModInfo> = installed
    .into_iter()
    .map(|m| ModInfo {
      name: m.manifest.name,
      version: m.manifest.version_number,
      dependencies: m.manifest.dependencies,
    })
    .collect();

  let count = mods.len();

  json(&ModsResponse {
    installed_mods: mods,
    count,
  })
}

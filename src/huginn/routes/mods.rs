use crate::fetch_mods;
use serde::Serialize;
use warp::reply::{json, Json};

#[derive(Serialize)]
#[serde(crate = "serde")]
pub struct ModInfo {
  pub name: String,
  pub version: Option<String>,
}

#[derive(Serialize)]
#[serde(crate = "serde")]
pub struct ModsResponse {
  pub installed_mods: Vec<ModInfo>,
  pub count: usize,
}

pub fn invoke() -> Json {
  let mods_data = fetch_mods();

  let mods: Vec<ModInfo> = mods_data
    .into_iter()
    .map(|(name, version)| ModInfo { name, version })
    .collect();

  let count = mods.len();

  json(&ModsResponse {
    installed_mods: mods,
    count,
  })
}

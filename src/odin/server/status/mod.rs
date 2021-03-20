mod bepinex_info;
mod jobs_info;

use crate::constants::{AUTO_BACKUP_JOB, AUTO_UPDATE_JOB};
use a2s::info::Info;
use a2s::A2SClient;
use bepinex_info::BepInExInfo;
use jobs_info::JobInfo;
use log::{debug, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerInfo {
  pub name: String,
  pub version: String,
  pub players: u8,
  pub max_players: u8,
  pub map: String,
  pub online: bool,
  pub bepinex: BepInExInfo,
  pub jobs: Vec<JobInfo>,
}

impl ServerInfo {
  pub fn new(address: &str) -> ServerInfo {
    let parsed_address = address.replace('"', "");
    debug!("Game IP {}", &parsed_address);
    let query_client = A2SClient::new().unwrap();
    match query_client.info(&parsed_address) {
      Ok(a2s_info) => ServerInfo::from(a2s_info),
      Err(_err) => {
        error!("Failed to request server information!");
        ServerInfo::default()
      }
    }
  }
}

impl From<String> for ServerInfo {
  fn from(address: String) -> ServerInfo {
    ServerInfo::new(&address)
  }
}

impl From<Info> for ServerInfo {
  fn from(info: Info) -> ServerInfo {
    let version = String::from(&info.clone().extended_server_info.keywords.unwrap());
    ServerInfo {
      name: info.name,
      version,
      players: info.players,
      max_players: info.max_players,
      map: info.map,
      online: true,
      bepinex: BepInExInfo::new(),
      jobs: vec![
        JobInfo::from(AUTO_UPDATE_JOB),
        JobInfo::from(AUTO_BACKUP_JOB),
      ],
    }
  }
}

impl Default for ServerInfo {
  fn default() -> ServerInfo {
    let unknown = String::from("Unknown");
    ServerInfo {
      name: unknown.clone(),
      version: unknown.clone(),
      players: 0,
      max_players: 0,
      map: unknown,
      online: false,
      bepinex: BepInExInfo::default(),
      jobs: vec![],
    }
  }
}

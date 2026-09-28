mod bepinex_info;
mod jobs_info;

use crate::utils::environment::fetch_var;
use crate::utils::scheduler_state::{load_scheduler_state, JobRuntimeInfo, SchedulerState};
use a2s::info::Info;
use a2s::A2SClient;
use bepinex_info::BepInExInfo;
use jobs_info::JobInfo;
use log::error;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerInfo {
  pub name: String,
  pub version: String,
  pub players: u8,
  pub max_players: u8,
  pub map: String,
  pub online: bool,
  pub bepinex: BepInExInfo,
  pub jobs: Vec<JobInfo>,
  pub scheduler_state: SchedulerState,
}

impl ServerInfo {
  fn scheduler_state_with_configured_jobs(jobs: &[JobInfo]) -> SchedulerState {
    let mut scheduler_state = load_scheduler_state();
    for job in jobs {
      if scheduler_state
        .jobs
        .iter()
        .any(|existing| existing.name == job.name)
      {
        continue;
      }
      scheduler_state.jobs.push(JobRuntimeInfo {
        name: job.name.clone(),
        ..JobRuntimeInfo::default()
      });
    }
    scheduler_state
  }

  pub fn new(address: SocketAddrV4) -> ServerInfo {
    let query_client = match A2SClient::new() {
      Ok(c) => c,
      Err(e) => {
        error!("Failed to initialize A2S client: {e}");
        return ServerInfo::offline();
      }
    };
    match query_client.info(address) {
      Ok(a2s_info) => ServerInfo::from(a2s_info),
      Err(err) => {
        error!("Failed to request server information from {address}: {err}");
        ServerInfo::offline()
      }
    }
  }
  pub fn offline() -> ServerInfo {
    let unknown = String::from("Unknown");
    let jobs = JobInfo::configured();
    ServerInfo {
      name: fetch_var("NAME", &unknown),
      version: unknown.clone(),
      players: 0,
      max_players: 0,
      map: fetch_var("NAME", &unknown),
      online: false,
      bepinex: BepInExInfo::disabled(),
      scheduler_state: Self::scheduler_state_with_configured_jobs(&jobs),
      jobs,
    }
  }
}

impl From<SocketAddrV4> for ServerInfo {
  fn from(address: SocketAddrV4) -> ServerInfo {
    ServerInfo::new(address)
  }
}

/// The version the server advertises in its A2S keywords field.
///
/// Whatever is on the other end of the query decides what to put here, and it is
/// free to send nothing. Unwrapping it meant a server that omitted the field took
/// down `/status` with a panic.
fn version_from_keywords(keywords: Option<&str>) -> String {
  match keywords {
    Some(keywords) if !keywords.trim().is_empty() => keywords.to_string(),
    _ => {
      error!("Server did not report a version in its A2S keywords field");
      String::from("Unknown")
    }
  }
}

impl From<Info> for ServerInfo {
  fn from(info: Info) -> ServerInfo {
    let version = version_from_keywords(info.extended_server_info.keywords.as_deref());
    let jobs = JobInfo::configured();
    ServerInfo {
      name: info.name,
      version,
      players: info.players,
      max_players: info.max_players,
      map: info.map,
      online: true,
      bepinex: BepInExInfo::new(),
      scheduler_state: Self::scheduler_state_with_configured_jobs(&jobs),
      jobs,
    }
  }
}

impl Display for ServerInfo {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let bepinex = &self.bepinex;
    let mut server_info = vec![
      format!("Name: {}", &self.name),
      format!("Players: {}/{}", &self.players, &self.max_players),
      format!("Map: {}", &self.map),
      format!("BepInEx Enabled: {}", bepinex.enabled),
    ];
    if bepinex.enabled {
      let mods: Vec<String> = bepinex.mods.iter().map(|m| String::from(&m.name)).collect();
      server_info.push(format!("BepInEx Mods: {}", mods.join(", ")));
    }
    write!(f, "{}", server_info.join("\n"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::constants::{AUTO_BACKUP_JOB, AUTO_UPDATE_JOB, SCHEDULED_RESTART_JOB};

  #[test]
  fn version_falls_back_when_the_server_sends_no_keywords() {
    assert_eq!(version_from_keywords(None), "Unknown");
  }

  #[test]
  fn version_falls_back_when_keywords_are_blank() {
    assert_eq!(version_from_keywords(Some("")), "Unknown");
    assert_eq!(version_from_keywords(Some("   ")), "Unknown");
  }

  #[test]
  fn version_is_the_reported_keywords() {
    assert_eq!(version_from_keywords(Some("0.217.46")), "0.217.46");
  }

  #[test]
  fn configured_jobs_cover_every_scheduled_job() {
    let names: Vec<String> = JobInfo::configured()
      .into_iter()
      .map(|job| job.name)
      .collect();
    assert_eq!(
      names,
      vec![AUTO_UPDATE_JOB, AUTO_BACKUP_JOB, SCHEDULED_RESTART_JOB]
    );
  }
}

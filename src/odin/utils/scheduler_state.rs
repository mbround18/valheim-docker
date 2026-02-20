use crate::utils::common_paths::log_directory;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobRuntimeInfo {
  pub name: String,
  pub last_started_at: Option<String>,
  pub last_finished_at: Option<String>,
  pub last_status: Option<String>,
  pub last_message: Option<String>,
  pub last_exit_code: Option<i32>,
  pub last_duration_ms: Option<u64>,
  pub run_count: u64,
  pub success_count: u64,
  pub failure_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchedulerState {
  pub updated_at: Option<String>,
  pub jobs: Vec<JobRuntimeInfo>,
}

pub fn scheduler_state_file() -> PathBuf {
  std::env::var("ODIN_SCHEDULER_STATE_FILE")
    .map(PathBuf::from)
    .unwrap_or_else(|_| PathBuf::from(log_directory()).join("jobs_state.json"))
}

pub fn load_scheduler_state() -> SchedulerState {
  let path = scheduler_state_file();
  match fs::read_to_string(&path) {
    Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
      warn!(
        "Failed to parse scheduler state at {}: {}. Resetting state.",
        path.display(),
        e
      );
      SchedulerState::default()
    }),
    Err(_) => SchedulerState::default(),
  }
}

pub fn save_scheduler_state(state: &SchedulerState) {
  let path = scheduler_state_file();
  if let Some(parent) = path.parent() {
    if let Err(e) = fs::create_dir_all(parent) {
      warn!(
        "Failed to create scheduler state directory {}: {}",
        parent.display(),
        e
      );
      return;
    }
  }

  let serialized = match serde_json::to_string_pretty(state) {
    Ok(s) => s,
    Err(e) => {
      warn!("Failed to serialize scheduler state: {}", e);
      return;
    }
  };

  if let Err(e) = fs::write(&path, serialized) {
    warn!(
      "Failed to write scheduler state file {}: {}",
      path.display(),
      e
    );
  } else {
    debug!("Wrote scheduler state to {}", path.display());
  }
}

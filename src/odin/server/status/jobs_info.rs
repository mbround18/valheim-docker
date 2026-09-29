use crate::constants::{AUTO_BACKUP_JOB, AUTO_UPDATE_JOB, SCHEDULED_RESTART_JOB};
use crate::utils::environment::fetch_var;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobInfo {
  pub name: String,
  pub enabled: bool,
  pub schedule: String,
}

impl JobInfo {
  /// Reads a job's state from the environment. Cannot fail: an unset job is a
  /// disabled one. `FromStr` delegates here so callers are not made to unwrap an
  /// `Infallible` result.
  pub fn new(job_name: &str) -> JobInfo {
    let sanitized_name = job_name.to_uppercase();
    let enabled: bool = fetch_var(&sanitized_name, "0").eq_ignore_ascii_case("1");
    let schedule = fetch_var(&format!("{sanitized_name}_SCHEDULE"), "never").replace('"', "");
    JobInfo {
      name: job_name.to_string(),
      enabled,
      schedule,
    }
  }

  /// The jobs odin knows how to schedule, in a fixed order.
  pub fn configured() -> Vec<JobInfo> {
    [AUTO_UPDATE_JOB, AUTO_BACKUP_JOB, SCHEDULED_RESTART_JOB]
      .iter()
      .map(|name| JobInfo::new(name))
      .collect()
  }
}

impl FromStr for JobInfo {
  type Err = std::convert::Infallible;

  fn from_str(job_name: &str) -> Result<JobInfo, std::convert::Infallible> {
    Ok(JobInfo::new(job_name))
  }
}

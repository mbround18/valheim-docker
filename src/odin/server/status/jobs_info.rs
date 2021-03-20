use crate::utils::environment::fetch_var;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct JobInfo {
  name: String,
  enabled: bool,
  schedule: String,
}

impl From<&str> for JobInfo {
  fn from(job_name: &str) -> Self {
    let sanitized_name = job_name.to_uppercase();
    let enabled: bool = fetch_var(&sanitized_name, "0").eq_ignore_ascii_case("1");
    let schedule = fetch_var(&format!("{}_SCHEDULE", &sanitized_name), "never").replace('"', "");
    JobInfo {
      name: job_name.to_string(),
      enabled,
      schedule,
    }
  }
}

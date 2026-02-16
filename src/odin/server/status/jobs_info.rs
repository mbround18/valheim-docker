use crate::utils::environment::fetch_var;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobInfo {
  pub name: String,
  pub enabled: bool,
  pub schedule: String,
}

impl FromStr for JobInfo {
  type Err = std::convert::Infallible;

  fn from_str(job_name: &str) -> Result<JobInfo, std::convert::Infallible> {
    let sanitized_name = job_name.to_uppercase();
    let enabled: bool = fetch_var(&sanitized_name, "0").eq_ignore_ascii_case("1");
    let schedule = fetch_var(&format!("{}_SCHEDULE", &sanitized_name), "never").replace('"', "");
    Ok(JobInfo {
      name: job_name.to_string(),
      enabled,
      schedule,
    })
  }
}

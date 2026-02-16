use chrono::{Datelike, Local, Timelike, Weekday};
use log::{debug, error, info, warn};
use std::collections::BTreeSet;
use std::env;
use std::net::SocketAddrV4;
use std::process::Command;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::server::ServerInfo;
use crate::utils::environment::fetch_var;
use crate::utils::scheduler_state::{
  load_scheduler_state, save_scheduler_state, JobRuntimeInfo, SchedulerState,
};

#[derive(Clone)]
struct CronSchedule {
  minute: BTreeSet<u32>,
  hour: BTreeSet<u32>,
  day_of_month: BTreeSet<u32>,
  month: BTreeSet<u32>,
  day_of_week: BTreeSet<u32>,
  day_of_month_is_wildcard: bool,
  day_of_week_is_wildcard: bool,
}

impl CronSchedule {
  fn parse(expr: &str) -> Result<Self, String> {
    let normalized = expr.trim().replace('"', "");
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() != 5 {
      return Err(format!(
        "invalid cron expression '{normalized}': expected 5 fields, got {}",
        parts.len()
      ));
    }
    let minute = parse_field(parts[0], 0, 59)?;
    let hour = parse_field(parts[1], 0, 23)?;
    let day_of_month = parse_field(parts[2], 1, 31)?;
    let month = parse_field(parts[3], 1, 12)?;
    let day_of_week = parse_dow_field(parts[4])?;
    Ok(Self {
      minute,
      hour,
      day_of_month,
      month,
      day_of_week,
      day_of_month_is_wildcard: is_wildcard(parts[2]),
      day_of_week_is_wildcard: is_wildcard(parts[4]),
    })
  }

  fn matches_now(&self) -> bool {
    let now = Local::now();
    self.matches(
      now.minute(),
      now.hour(),
      now.day(),
      now.month(),
      now.weekday(),
    )
  }

  fn matches(
    &self,
    minute: u32,
    hour: u32,
    day_of_month: u32,
    month: u32,
    weekday: Weekday,
  ) -> bool {
    if !self.minute.contains(&minute) {
      return false;
    }
    if !self.hour.contains(&hour) {
      return false;
    }
    if !self.month.contains(&month) {
      return false;
    }
    let dow = weekday.num_days_from_sunday();
    let dom_matches = self.day_of_month.contains(&day_of_month);
    let dow_matches = self.day_of_week.contains(&dow);
    // Cron compatibility: if both DOM and DOW are restricted, either one may match.
    if !self.day_of_month_is_wildcard && !self.day_of_week_is_wildcard {
      dom_matches || dow_matches
    } else {
      dom_matches && dow_matches
    }
  }
}

fn is_wildcard(field: &str) -> bool {
  field.trim() == "*"
}

fn parse_dow_field(field: &str) -> Result<BTreeSet<u32>, String> {
  let mut out = parse_field(field, 0, 7)?;
  if out.contains(&7) {
    out.remove(&7);
    out.insert(0);
  }
  Ok(out)
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<BTreeSet<u32>, String> {
  let field = field.trim();
  if field.is_empty() {
    return Err("empty cron field".to_string());
  }

  let mut values = BTreeSet::new();
  for piece in field.split(',') {
    parse_piece(piece.trim(), min, max, &mut values)?;
  }

  if values.is_empty() {
    return Err(format!("cron field '{field}' resolved to no values"));
  }
  Ok(values)
}

fn parse_piece(piece: &str, min: u32, max: u32, values: &mut BTreeSet<u32>) -> Result<(), String> {
  if piece == "*" {
    for value in min..=max {
      values.insert(value);
    }
    return Ok(());
  }

  let (range_expr, step) = if let Some((lhs, rhs)) = piece.split_once('/') {
    let step = rhs
      .parse::<u32>()
      .map_err(|_| format!("invalid step value '{rhs}'"))?;
    if step == 0 {
      return Err("step cannot be zero".to_string());
    }
    (lhs, step)
  } else {
    (piece, 1)
  };

  if range_expr == "*" {
    let mut value = min;
    while value <= max {
      values.insert(value);
      match value.checked_add(step) {
        Some(next) => value = next,
        None => break,
      }
    }
    return Ok(());
  }

  let (start, end) = if let Some((start, end)) = range_expr.split_once('-') {
    let start = parse_u32_in_range(start, min, max)?;
    let end = parse_u32_in_range(end, min, max)?;
    if end < start {
      return Err(format!("invalid range '{range_expr}'"));
    }
    (start, end)
  } else {
    let value = parse_u32_in_range(range_expr, min, max)?;
    (value, value)
  };

  let mut value = start;
  while value <= end {
    values.insert(value);
    match value.checked_add(step) {
      Some(next) => value = next,
      None => break,
    }
  }
  Ok(())
}

fn parse_u32_in_range(input: &str, min: u32, max: u32) -> Result<u32, String> {
  let value = input
    .parse::<u32>()
    .map_err(|_| format!("invalid numeric value '{input}'"))?;
  if value < min || value > max {
    return Err(format!("value '{value}' out of range [{min}, {max}]"));
  }
  Ok(value)
}

struct JobSpec {
  name: &'static str,
  enabled_env: &'static str,
  schedule_env: &'static str,
  default_schedule: &'static str,
  action: fn() -> JobRunResult,
}

#[derive(Debug)]
struct JobRunResult {
  ok: bool,
  message: String,
  exit_code: Option<i32>,
}

impl JobRunResult {
  fn success(message: impl Into<String>) -> Self {
    Self {
      ok: true,
      message: message.into(),
      exit_code: Some(0),
    }
  }

  fn failure(message: impl Into<String>, exit_code: Option<i32>) -> Self {
    Self {
      ok: false,
      message: message.into(),
      exit_code,
    }
  }
}

fn auto_update_job() -> JobRunResult {
  let update_check = run_odin(&["update", "--check"]);
  match update_check {
    Ok(0) => {
      if fetch_var("AUTO_UPDATE_PAUSE_WITH_PLAYERS", "0") == "1" {
        let public = fetch_var("PUBLIC", "0");
        if public == "0" {
          warn!(
            "AUTO_UPDATE_PAUSE_WITH_PLAYERS ignored because PUBLIC=0; cannot query public player count"
          );
        } else if players_online() > 0 {
          info!("Skipping auto update: players are online");
          return JobRunResult::success("Skipped auto update: players online");
        }
      }

      info!("Auto update triggered");
      if fetch_var("AUTO_BACKUP_ON_UPDATE", "0") == "1" {
        let backup = run_backup(Some("pre-update-backup"));
        if !backup.ok {
          return JobRunResult::failure(
            format!(
              "Auto update aborted because pre-update backup failed: {}",
              backup.message
            ),
            backup.exit_code,
          );
        }
      }
      match run_odin(&["update"]) {
        Ok(0) => JobRunResult::success("Auto update completed"),
        Ok(code) => {
          JobRunResult::failure(format!("Auto update failed with code {code}"), Some(code))
        }
        Err(e) => JobRunResult::failure(format!("Auto update failed: {e}"), None),
      }
    }
    Ok(10) => {
      debug!("Auto update check: no update available");
      JobRunResult::success("No update available")
    }
    Ok(code) => JobRunResult::failure(
      format!("Auto update check failed with code {code}"),
      Some(code),
    ),
    Err(e) => JobRunResult::failure(format!("Auto update check failed: {e}"), None),
  }
}

fn auto_backup_job() -> JobRunResult {
  run_backup(None)
}

fn scheduled_restart_job() -> JobRunResult {
  info!("Scheduled restart triggered");
  match run_odin(&["stop"]) {
    Ok(0) => {}
    Ok(code) => {
      return JobRunResult::failure(
        format!("Scheduled restart failed to stop server with code {code}"),
        Some(code),
      );
    }
    Err(e) => {
      return JobRunResult::failure(
        format!("Scheduled restart failed to stop server: {e}"),
        None,
      );
    }
  }
  thread::sleep(Duration::from_secs(5));
  match run_odin(&["start"]) {
    Ok(0) => JobRunResult::success("Scheduled restart completed"),
    Ok(code) => JobRunResult::failure(
      format!("Scheduled restart failed to start server with code {code}"),
      Some(code),
    ),
    Err(e) => JobRunResult::failure(
      format!("Scheduled restart failed to start server: {e}"),
      None,
    ),
  }
}

fn run_backup(suffix: Option<&str>) -> JobRunResult {
  if fetch_var("AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS", "0") == "1" {
    let public = fetch_var("PUBLIC", "0");
    let crossplay = fetch_var("ENABLE_CROSSPLAY", "0");
    if public == "0" || crossplay == "1" {
      warn!(
        "AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS ignored because PUBLIC=0 or ENABLE_CROSSPLAY=1; cannot query Steam API reliably"
      );
    } else if players_online() == 0 {
      info!("Skipping backup: no players are online");
      return JobRunResult::success("Skipped backup: no players online");
    }
  }

  if fetch_var("AUTO_BACKUP_REMOVE_OLD", "0") == "1" {
    prune_old_backups();
  }

  let timestamp = Local::now().format("%Y%m%d-%H%M%S");
  let label = suffix.unwrap_or("backup");
  let output = format!("/home/steam/backups/{timestamp}-{label}.tar.gz");
  let input = "/home/steam/.config/unity3d/IronGate/Valheim";

  let nice_level = fetch_var("AUTO_BACKUP_NICE_LEVEL", "0")
    .parse::<i32>()
    .unwrap_or(0);
  if (1..=19).contains(&nice_level) {
    let exe = match env::current_exe() {
      Ok(exe) => exe,
      Err(e) => {
        return JobRunResult::failure(
          format!("Backup failed to locate odin executable: {e}"),
          None,
        );
      }
    };
    let status = Command::new("nice")
      .args(["-n", &nice_level.to_string()])
      .arg(exe)
      .args(["backup", input, &output])
      .status();
    match status {
      Ok(s) if s.success() => return JobRunResult::success(format!("Backup complete: {output}")),
      Ok(s) => {
        return JobRunResult::failure(
          format!("Backup failed with exit code {:?}", s.code()),
          s.code(),
        );
      }
      Err(e) => {
        return JobRunResult::failure(format!("Backup failed to launch with nice: {e}"), None)
      }
    }
  }

  match run_odin(&["backup", input, &output]) {
    Ok(0) => JobRunResult::success(format!("Backup complete: {output}")),
    Ok(code) => JobRunResult::failure(format!("Backup failed with code {code}"), Some(code)),
    Err(e) => JobRunResult::failure(format!("Backup failed: {e}"), None),
  }
}

fn prune_old_backups() {
  use std::fs;
  use std::time::{Duration as StdDuration, SystemTime};

  let days = fetch_var("AUTO_BACKUP_DAYS_TO_LIVE", "5")
    .parse::<u64>()
    .unwrap_or(5)
    .max(1);
  let max_age = StdDuration::from_secs(days * 24 * 60 * 60);
  let now = SystemTime::now();
  let backup_dir = "/home/steam/backups";

  let entries = match fs::read_dir(backup_dir) {
    Ok(entries) => entries,
    Err(e) => {
      warn!("Could not read backup dir '{backup_dir}': {e}");
      return;
    }
  };

  for entry in entries.flatten() {
    let path = entry.path();
    let is_file = path.is_file();
    if !is_file {
      continue;
    }
    let modified = match entry.metadata().and_then(|m| m.modified()) {
      Ok(m) => m,
      Err(_) => continue,
    };
    let age = match now.duration_since(modified) {
      Ok(age) => age,
      Err(_) => continue,
    };
    if age > max_age {
      match fs::remove_file(&path) {
        Ok(_) => info!("Removed old backup: {}", path.display()),
        Err(e) => warn!("Failed to remove old backup {}: {e}", path.display()),
      }
    }
  }
}

fn players_online() -> u8 {
  let address = fetch_var("ADDRESS", "127.0.0.1:2457");
  match SocketAddrV4::from_str(&address) {
    Ok(parsed) => ServerInfo::from(parsed).players,
    Err(e) => {
      warn!("Failed to parse ADDRESS '{address}': {e}");
      0
    }
  }
}

fn run_odin(args: &[&str]) -> Result<i32, String> {
  let exe = env::current_exe().map_err(|e| format!("failed to locate odin executable: {e}"))?;
  let status = Command::new(exe)
    .args(args)
    .status()
    .map_err(|e| format!("failed to launch odin {:?}: {e}", args))?;
  Ok(status.code().unwrap_or(1))
}

fn run_job_tick(job_specs: &[JobSpec], state: &mut SchedulerState) {
  for job in job_specs {
    let enabled = fetch_var(job.enabled_env, "0") == "1";
    if !enabled {
      continue;
    }
    let schedule_text = fetch_var(job.schedule_env, job.default_schedule);
    match CronSchedule::parse(&schedule_text) {
      Ok(schedule) => {
        if schedule.matches_now() {
          info!("Running scheduled job {}", job.name);
          let started = Local::now();
          let result = (job.action)();
          let finished = Local::now();
          update_state_for_job(
            state,
            job.name,
            started.to_rfc3339(),
            finished.to_rfc3339(),
            result,
          );
        }
      }
      Err(e) => {
        error!(
          "Invalid schedule for {} from {}='{}': {}",
          job.name, job.schedule_env, schedule_text, e
        );
      }
    }
  }
  state.updated_at = Some(Local::now().to_rfc3339());
  save_scheduler_state(state);
}

fn update_state_for_job(
  state: &mut SchedulerState,
  job_name: &str,
  started_at: String,
  finished_at: String,
  result: JobRunResult,
) {
  let duration_ms = chrono::DateTime::parse_from_rfc3339(&finished_at)
    .ok()
    .zip(chrono::DateTime::parse_from_rfc3339(&started_at).ok())
    .map(|(end, start)| (end - start).num_milliseconds().max(0) as u128);

  let idx = state.jobs.iter().position(|j| j.name == job_name);
  let job = if let Some(i) = idx {
    &mut state.jobs[i]
  } else {
    state.jobs.push(JobRuntimeInfo {
      name: job_name.to_string(),
      ..JobRuntimeInfo::default()
    });
    state.jobs.last_mut().expect("state.jobs just pushed")
  };

  job.last_started_at = Some(started_at);
  job.last_finished_at = Some(finished_at);
  job.last_status = Some(if result.ok { "success" } else { "failure" }.to_string());
  job.last_message = Some(result.message);
  job.last_exit_code = result.exit_code;
  job.last_duration_ms = duration_ms;
  job.run_count += 1;
  if result.ok {
    job.success_count += 1;
  } else {
    job.failure_count += 1;
  }
}

pub fn invoke(once: bool) {
  let jobs = vec![
    JobSpec {
      name: "AUTO_UPDATE",
      enabled_env: "AUTO_UPDATE",
      schedule_env: "AUTO_UPDATE_SCHEDULE",
      default_schedule: "0 1 * * *",
      action: auto_update_job,
    },
    JobSpec {
      name: "AUTO_BACKUP",
      enabled_env: "AUTO_BACKUP",
      schedule_env: "AUTO_BACKUP_SCHEDULE",
      default_schedule: "*/15 * * * *",
      action: auto_backup_job,
    },
    JobSpec {
      name: "SCHEDULED_RESTART",
      enabled_env: "SCHEDULED_RESTART",
      schedule_env: "SCHEDULED_RESTART_SCHEDULE",
      default_schedule: "0 2 * * *",
      action: scheduled_restart_job,
    },
  ];
  let mut state = load_scheduler_state();

  if once {
    run_job_tick(&jobs, &mut state);
    return;
  }

  info!("Starting Odin scheduler loop");
  let mut last_minute_key = String::new();
  loop {
    let now = Local::now();
    let minute_key = now.format("%Y-%m-%d %H:%M").to_string();
    if minute_key != last_minute_key {
      last_minute_key = minute_key;
      run_job_tick(&jobs, &mut state);
    }
    thread::sleep(Duration::from_secs(5));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_simple_wildcard_schedule() {
    let parsed = CronSchedule::parse("*/15 * * * *").unwrap();
    assert!(parsed.minute.contains(&0));
    assert!(parsed.minute.contains(&15));
    assert!(parsed.minute.contains(&30));
    assert!(parsed.minute.contains(&45));
  }

  #[test]
  fn parse_list_and_range_schedule() {
    let parsed = CronSchedule::parse("0 1,3,5 1-5 * 1-5").unwrap();
    assert!(parsed.hour.contains(&1));
    assert!(parsed.hour.contains(&3));
    assert!(parsed.hour.contains(&5));
    assert!(parsed.day_of_month.contains(&1));
    assert!(parsed.day_of_month.contains(&5));
    assert!(!parsed.day_of_month.contains(&6));
  }

  #[test]
  fn normalize_sunday_7_to_0() {
    let parsed = CronSchedule::parse("0 0 * * 7").unwrap();
    assert!(parsed.day_of_week.contains(&0));
    assert!(!parsed.day_of_week.contains(&7));
  }
}

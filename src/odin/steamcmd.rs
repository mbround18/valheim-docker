use crate::executable::find_command;
use crate::utils::environment::is_env_var_truthy;
use log::{debug, error, info, warn};
use std::{
  env, io,
  process::{exit, Command, ExitStatus, Output, Stdio},
  thread,
  time::Duration,
};

const STEAMCMD_EXE: &str = "/home/steam/steamcmd/steamcmd.sh";

#[derive(Debug, Clone, Copy)]
struct RetryConfig {
  attempts: u32,
  base_delay_secs: u64,
}

impl RetryConfig {
  fn from_env() -> Self {
    let attempts = env::var("STEAMCMD_RETRY_ATTEMPTS")
      .ok()
      .and_then(|v| v.parse::<u32>().ok())
      .unwrap_or(3)
      .max(1);
    let base_delay_secs = env::var("STEAMCMD_RETRY_BASE_DELAY_SECS")
      .ok()
      .and_then(|v| v.parse::<u64>().ok())
      .unwrap_or(5)
      .max(1);
    Self {
      attempts,
      base_delay_secs,
    }
  }
}

/// With `STEAMCMD_64BIT` truthy, have Valve's `steamcmd.sh` run the 64-bit binary
/// (`linux64/steamcmd`) instead of the default 32-bit one. Runtimes without 32-bit
/// support (gVisor, Kata) need this. Set per command so `odin` started any way
/// (entrypoint, `docker exec`, scheduled jobs) behaves the same.
fn apply_steam_platform(command: &mut Command, use_64bit: bool) {
  if use_64bit {
    command.env("STEAM_PLATFORM", "linux64");
  }
}

pub fn steamcmd_command() -> Command {
  let mut steamcmd = find_steamcmd();
  apply_steam_platform(&mut steamcmd, is_env_var_truthy("STEAMCMD_64BIT"));
  steamcmd
}

fn find_steamcmd() -> Command {
  if let Some(steamcmd) = find_command("steamcmd") {
    info!("steamcmd found in path");
    steamcmd
  } else {
    error!("Checking for script under steam user.");
    if let Some(steamcmd) = find_command(STEAMCMD_EXE) {
      info!("Using steamcmd script at {STEAMCMD_EXE}");
      steamcmd
    } else {
      error!("\nSteamCMD Executable Not Found! \nPlease install steamcmd... \nhttps://developer.valvesoftware.com/wiki/SteamCMD\n");
      exit(1);
    }
  }
}

pub fn run_with_retries(args: &[String]) -> io::Result<ExitStatus> {
  let retry = RetryConfig::from_env();
  let mut attempt = 1;
  let mut delay_secs = retry.base_delay_secs;

  loop {
    let mut steamcmd = steamcmd_command();
    steamcmd
      .args(args)
      .arg("+quit")
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit());
    debug!(
      "Launching steamcmd attempt {attempt}/{}: {steamcmd:#?}",
      retry.attempts
    );

    match steamcmd.status() {
      Ok(status) if status.success() => return Ok(status),
      Ok(status) => {
        if attempt >= retry.attempts {
          return Ok(status);
        }
        warn!(
          "steamcmd failed on attempt {attempt}/{} with exit code {:?}; retrying in {}s",
          retry.attempts,
          status.code(),
          delay_secs
        );
      }
      Err(e) => {
        if attempt >= retry.attempts {
          return Err(e);
        }
        warn!(
          "steamcmd execution error on attempt {attempt}/{}: {e}; retrying in {}s",
          retry.attempts, delay_secs
        );
      }
    }

    thread::sleep(Duration::from_secs(delay_secs));
    delay_secs = delay_secs.saturating_mul(2);
    attempt += 1;
  }
}

pub fn output_with_retries(args: &[String]) -> io::Result<Output> {
  let retry = RetryConfig::from_env();
  let mut attempt = 1;
  let mut delay_secs = retry.base_delay_secs;

  loop {
    let mut steamcmd = steamcmd_command();
    steamcmd.args(args);
    debug!(
      "Launching steamcmd output attempt {attempt}/{}: {steamcmd:#?}",
      retry.attempts
    );

    match steamcmd.output() {
      Ok(output) if output.status.success() => return Ok(output),
      Ok(output) => {
        if attempt >= retry.attempts {
          return Ok(output);
        }
        warn!(
          "steamcmd output command failed on attempt {attempt}/{} with exit code {:?}; retrying in {}s",
          retry.attempts,
          output.status.code(),
          delay_secs
        );
      }
      Err(e) => {
        if attempt >= retry.attempts {
          return Err(e);
        }
        warn!(
          "steamcmd output execution error on attempt {attempt}/{}: {e}; retrying in {}s",
          retry.attempts, delay_secs
        );
      }
    }

    thread::sleep(Duration::from_secs(delay_secs));
    delay_secs = delay_secs.saturating_mul(2);
    attempt += 1;
  }
}

#[cfg(test)]
mod steam_platform_tests {
  use super::apply_steam_platform;
  use std::{ffi::OsStr, process::Command};

  fn steam_platform(command: &Command) -> Option<Option<&OsStr>> {
    command
      .get_envs()
      .find(|(key, _)| *key == OsStr::new("STEAM_PLATFORM"))
      .map(|(_, value)| value)
  }

  #[test]
  fn sets_linux64_when_enabled() {
    let mut command = Command::new("steamcmd");
    apply_steam_platform(&mut command, true);
    assert_eq!(steam_platform(&command), Some(Some(OsStr::new("linux64"))));
  }

  #[test]
  fn leaves_platform_alone_when_disabled() {
    let mut command = Command::new("steamcmd");
    apply_steam_platform(&mut command, false);
    assert_eq!(steam_platform(&command), None);
  }
}

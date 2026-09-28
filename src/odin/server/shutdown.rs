use log::{debug, error, info, warn};

use std::{
  thread,
  time::{Duration, Instant},
};
use thiserror::Error;

use crate::server::process::ServerProcess;
use crate::utils::environment::fetch_var;

/// How long to wait for the server to exit on its own after `SIGINT`, in seconds.
/// Generous by default: the server saves the world on the way out, and a big world
/// on slow storage takes a while.
pub const SHUTDOWN_GRACE_TIMEOUT_VAR: &str = "SHUTDOWN_GRACE_TIMEOUT_SECS";
/// How long to wait after escalating to `SIGKILL` before giving up, in seconds.
pub const SHUTDOWN_KILL_TIMEOUT_VAR: &str = "SHUTDOWN_KILL_TIMEOUT_SECS";

const DEFAULT_GRACE_TIMEOUT_SECS: u64 = 120;
const DEFAULT_KILL_TIMEOUT_SECS: u64 = 30;
const POLL_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShutdownError {
  #[error(
    "Valheim is still running {grace}s after SIGINT and {kill}s after SIGKILL. \
     odin most likely does not have permission to signal it: check that the server \
     and odin run as the same user, or raise {grace_var}."
  )]
  StillRunning {
    grace: u64,
    kill: u64,
    grace_var: &'static str,
  },
}

/// Stops the Valheim server and waits for it to actually be gone.
///
/// Bounded at every step. This used to loop forever when the signal never landed,
/// so a cron'd `odin stop` against a server owned by another user piled up a new
/// stuck process on every run (#337). Now the wait is capped, an ignored `SIGINT`
/// escalates to `SIGKILL`, and a server that outlives both is reported to the
/// caller rather than blocking it.
pub fn blocking_shutdown() -> Result<(), ShutdownError> {
  let grace = timeout_secs(SHUTDOWN_GRACE_TIMEOUT_VAR, DEFAULT_GRACE_TIMEOUT_SECS);
  let kill = timeout_secs(SHUTDOWN_KILL_TIMEOUT_VAR, DEFAULT_KILL_TIMEOUT_SECS);

  let mut server_process = ServerProcess::new();
  if !server_process.are_process_running() {
    info!("Valheim is not running; nothing to shut down.");
    return Ok(());
  }

  let report = server_process.send_interrupt();
  if report.refused > 0 {
    warn!(
      "{} Valheim process(es) refused SIGINT. Continuing to wait, but this usually \
       means odin is running as a different user than the server.",
      report.refused
    );
  }

  if wait_for_exit(Duration::from_secs(grace)) {
    info!("Valheim process has been stopped successfully!");
    return Ok(());
  }

  warn!("Valheim did not stop within {grace}s of SIGINT; escalating to SIGKILL.");
  let report = ServerProcess::new().send_kill();
  if report.refused > 0 {
    error!("{} Valheim process(es) refused SIGKILL.", report.refused);
  }

  if wait_for_exit(Duration::from_secs(kill)) {
    warn!("Valheim process was killed. The world may not have been saved cleanly.");
    return Ok(());
  }

  Err(ShutdownError::StillRunning {
    grace,
    kill,
    grace_var: SHUTDOWN_GRACE_TIMEOUT_VAR,
  })
}

fn timeout_secs(name: &str, default: u64) -> u64 {
  fetch_var(name, "")
    .parse::<u64>()
    .ok()
    .filter(|secs| *secs > 0)
    .unwrap_or(default)
}

/// Polls for the server to disappear. A fresh [`ServerProcess`] per check, because
/// only a newly built process table reliably drops entries that have since exited.
fn wait_for_exit(timeout: Duration) -> bool {
  wait_until(
    || {
      debug!("Checking if valheim is still running.");
      !ServerProcess::new().are_process_running()
    },
    timeout,
    POLL_INTERVAL,
  )
}

/// Runs `done` now and then every `poll` until it returns true or `timeout` elapses.
/// Always checks once, so a zero timeout still gets one look.
fn wait_until<F>(mut done: F, timeout: Duration, poll: Duration) -> bool
where
  F: FnMut() -> bool,
{
  let deadline = Instant::now() + timeout;
  loop {
    if done() {
      return true;
    }
    let now = Instant::now();
    if now >= deadline {
      return false;
    }
    let nap = poll.min(deadline - now);
    debug!("Sleeping for {nap:?} to wait for process to stop.");
    thread::sleep(nap);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::cell::Cell;
  use std::env::{remove_var, set_var};

  #[test]
  fn wait_until_returns_immediately_when_already_done() {
    let calls = Cell::new(0);
    let started = Instant::now();
    let stopped = wait_until(
      || {
        calls.set(calls.get() + 1);
        true
      },
      Duration::from_secs(60),
      Duration::from_secs(5),
    );
    assert!(stopped);
    assert_eq!(calls.get(), 1, "should not poll again once done");
    assert!(
      started.elapsed() < Duration::from_secs(1),
      "should not sleep"
    );
  }

  #[test]
  fn wait_until_gives_up_at_the_deadline() {
    let started = Instant::now();
    let stopped = wait_until(
      || false,
      Duration::from_millis(60),
      Duration::from_millis(20),
    );
    assert!(
      !stopped,
      "a process that never exits must not block forever"
    );
    assert!(
      started.elapsed() < Duration::from_secs(5),
      "must return at the deadline, not spin"
    );
  }

  #[test]
  fn wait_until_checks_once_even_with_a_zero_timeout() {
    let calls = Cell::new(0);
    let stopped = wait_until(
      || {
        calls.set(calls.get() + 1);
        false
      },
      Duration::ZERO,
      Duration::from_secs(5),
    );
    assert!(!stopped);
    assert_eq!(calls.get(), 1);
  }

  #[test]
  fn wait_until_succeeds_on_a_later_poll() {
    let calls = Cell::new(0);
    let stopped = wait_until(
      || {
        calls.set(calls.get() + 1);
        calls.get() >= 3
      },
      Duration::from_secs(60),
      Duration::from_millis(1),
    );
    assert!(stopped);
    assert_eq!(calls.get(), 3);
  }

  #[test]
  #[serial]
  fn timeout_secs_defaults_when_unset_or_nonsense() {
    unsafe { remove_var(SHUTDOWN_GRACE_TIMEOUT_VAR) };
    assert_eq!(timeout_secs(SHUTDOWN_GRACE_TIMEOUT_VAR, 120), 120);

    for bad in ["", "0", "-5", "soon"] {
      unsafe { set_var(SHUTDOWN_GRACE_TIMEOUT_VAR, bad) };
      assert_eq!(
        timeout_secs(SHUTDOWN_GRACE_TIMEOUT_VAR, 120),
        120,
        "{bad:?} should fall back to the default"
      );
    }
    unsafe { remove_var(SHUTDOWN_GRACE_TIMEOUT_VAR) };
  }

  #[test]
  #[serial]
  fn timeout_secs_honors_an_override() {
    unsafe { set_var(SHUTDOWN_KILL_TIMEOUT_VAR, "45") };
    assert_eq!(timeout_secs(SHUTDOWN_KILL_TIMEOUT_VAR, 30), 45);
    unsafe { remove_var(SHUTDOWN_KILL_TIMEOUT_VAR) };
  }

  #[test]
  fn still_running_error_names_the_knob_to_turn() {
    let message = ShutdownError::StillRunning {
      grace: 120,
      kill: 30,
      grace_var: SHUTDOWN_GRACE_TIMEOUT_VAR,
    }
    .to_string();
    assert!(message.contains("120s after SIGINT"));
    assert!(message.contains("30s after SIGKILL"));
    assert!(message.contains(SHUTDOWN_GRACE_TIMEOUT_VAR));
  }
}

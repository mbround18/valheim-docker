use crate::notifications::enums::{
  event_status::EventStatus, notification_event::NotificationEvent,
};
use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};

/// Valheim logs a failed master-server registration as `Game server connected failed`,
/// which has the success line as its prefix, so the failure has to be tested first.
const CONNECT_FAILED: &str = "Game server connected failed";
const CONNECT_OK: &str = "Game server connected";
const STEAM_DESTROY: &str = "Steam manager on destroy";

/// Whether the last master-server registration attempt failed. Valheim retries every
/// minute or so, so only the first failure of a streak sends a webhook.
static REGISTRATION_FAILING: AtomicBool = AtomicBool::new(false);

/// A launch/shutdown milestone parsed from one server log line.
#[derive(Debug, PartialEq, Eq)]
enum LaunchProbe {
  /// Registered with the Steam master server; the server is up and listed.
  Connected,
  /// Could not reach the Steam master server. The server keeps running, it just does not
  /// appear in the public server browser.
  ConnectFailed,
  Stopped,
}

fn classify(line: &str) -> Option<LaunchProbe> {
  if line.contains(CONNECT_FAILED) {
    Some(LaunchProbe::ConnectFailed)
  } else if line.contains(CONNECT_OK) {
    Some(LaunchProbe::Connected)
  } else if line.contains(STEAM_DESTROY) {
    Some(LaunchProbe::Stopped)
  } else {
    None
  }
}

pub fn handle_launch_probes(line: &str) {
  match classify(line) {
    Some(LaunchProbe::ConnectFailed) => {
      if REGISTRATION_FAILING.swap(true, Ordering::SeqCst) {
        debug!("Still failing to register with the Steam master server.");
      } else {
        error!("Failed to register with the Steam master server. The server is running but will not show up in the public server browser.");
        NotificationEvent::Start(EventStatus::Failed).send_notification(Some(
          "Server could not register with the Steam master server. It is running, but not listed in the public server browser.".to_string(),
        ));
      }
    }
    Some(LaunchProbe::Connected) => {
      debug!("Detected '{CONNECT_OK}'. Sending Start notification.");
      REGISTRATION_FAILING.store(false, Ordering::SeqCst);
      NotificationEvent::Start(EventStatus::Successful).send_notification(None);
    }
    Some(LaunchProbe::Stopped) => {
      debug!("Detected '{STEAM_DESTROY}'. Sending Stop notification.");
      NotificationEvent::Stop(EventStatus::Successful).send_notification(None);
    }
    None => {}
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;

  const FAILED_LINE: &str = "09/15/2026 18:03:41: Game server connected failed";
  const CONNECTED_LINE: &str = "09/15/2026 18:16:18: Game server connected";
  const DESTROY_LINE: &str = "09/15/2026 18:20:02: Steam manager on destroy";

  /// The reason this bug existed: the failure line contains the success line.
  #[test]
  fn a_failed_registration_is_not_read_as_a_successful_one() {
    assert!(FAILED_LINE.contains(CONNECT_OK));
    assert_eq!(classify(FAILED_LINE), Some(LaunchProbe::ConnectFailed));
    assert_eq!(classify(CONNECTED_LINE), Some(LaunchProbe::Connected));
    assert_eq!(classify(DESTROY_LINE), Some(LaunchProbe::Stopped));
    assert_eq!(
      classify("09/15/2026 18:16:18: World save (5/5) done."),
      None
    );
  }

  #[test]
  #[serial]
  fn repeated_failures_only_notify_once_until_it_connects() {
    std::env::remove_var("WEBHOOK_URL");
    REGISTRATION_FAILING.store(false, Ordering::SeqCst);

    handle_launch_probes(FAILED_LINE);
    assert!(REGISTRATION_FAILING.load(Ordering::SeqCst));
    handle_launch_probes(FAILED_LINE);
    assert!(REGISTRATION_FAILING.load(Ordering::SeqCst));

    handle_launch_probes(CONNECTED_LINE);
    assert!(!REGISTRATION_FAILING.load(Ordering::SeqCst));
  }
}

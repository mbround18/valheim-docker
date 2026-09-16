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
  use mockito::{Matcher, Mock, Server, ServerGuard};
  use serial_test::serial;

  /// Lines as Valheim writes them. The failure is the one from #1534, and it contains the
  /// success line, which is what made every failure report a successful start.
  const FAILED: &str = "09/15/2026 18:03:41: Game server connected failed";
  const CONNECTED: &str = "09/15/2026 18:16:18: Game server connected";
  const DESTROY: &str = "09/15/2026 18:20:02: Steam manager on destroy";

  /// Points the notifier at a mock webhook with both status switches on, and resets the
  /// streak so every test starts from a server that has just come up. Restores the
  /// environment on drop, since these are process-wide.
  struct Webhook {
    server: ServerGuard,
  }

  impl Webhook {
    fn new() -> Self {
      let server = Server::new();
      std::env::set_var("NAME", "probe-test-server");
      std::env::set_var("WEBHOOK_URL", server.url());
      std::env::set_var("WEBHOOK_STATUS_SUCCESSFUL", "1");
      std::env::set_var("WEBHOOK_STATUS_FAILED", "1");
      REGISTRATION_FAILING.store(false, Ordering::SeqCst);
      Webhook { server }
    }

    /// Expects exactly `times` webhooks for one `<name> <status>` event, matched on the
    /// payload the receiver actually sees.
    fn expect(&mut self, name: &str, status: &str, times: usize) -> Mock {
      self
        .server
        .mock("POST", "/")
        .match_body(Matcher::PartialJsonString(format!(
          r#"{{"event_type":{{"name":"{name}","status":"{status}"}}}}"#
        )))
        .with_status(204)
        .expect(times)
        .create()
    }
  }

  impl Drop for Webhook {
    fn drop(&mut self) {
      for var in [
        "NAME",
        "WEBHOOK_URL",
        "WEBHOOK_STATUS_SUCCESSFUL",
        "WEBHOOK_STATUS_FAILED",
      ] {
        std::env::remove_var(var);
      }
      REGISTRATION_FAILING.store(false, Ordering::SeqCst);
    }
  }

  /// The root cause: the failure line has the success line as a prefix, so `contains` alone
  /// cannot tell them apart and the order of the checks is what matters.
  #[test]
  fn the_failure_line_contains_the_success_line() {
    assert!(FAILED.contains(CONNECT_OK));
    assert_eq!(classify(FAILED), Some(LaunchProbe::ConnectFailed));
    assert_eq!(classify(CONNECTED), Some(LaunchProbe::Connected));
    assert_eq!(classify(DESTROY), Some(LaunchProbe::Stopped));
  }

  #[test]
  fn ordinary_server_output_is_not_a_probe() {
    for line in [
      "",
      "09/15/2026 18:03:41: World save (527/527) done. Total time [143ms]",
      "09/15/2026 18:03:41: Got character ZDOID from Viking : 2130425389:1",
      "[UnityMemory] Configuration Parameters - Can be set up in boot.config",
    ] {
      assert_eq!(classify(line), None, "{line:?}");
    }
  }

  /// #1534 as reported: a ~13 minute window where the server could not reach the Steam
  /// master server produced one `Start Successful` webhook per failure line, which reads
  /// like a crash loop. The whole window is worth one `Start Failed`, and the line that
  /// finally succeeds is worth one `Start Successful`.
  #[test]
  #[serial]
  fn a_failed_registration_reports_a_failure_and_never_a_success() {
    let mut webhook = Webhook::new();
    let failed = webhook.expect("Start", "Failed", 1);
    let successful = webhook.expect("Start", "Successful", 1);

    for _ in 0..13 {
      handle_launch_probes(FAILED);
    }
    handle_launch_probes(CONNECTED);

    failed.assert();
    successful.assert();
  }

  /// Registration can drop again later in the run, and that outage has to be reported too.
  #[test]
  #[serial]
  fn the_streak_resets_so_a_later_outage_notifies_again() {
    let mut webhook = Webhook::new();
    let failed = webhook.expect("Start", "Failed", 2);
    let successful = webhook.expect("Start", "Successful", 2);

    handle_launch_probes(FAILED);
    handle_launch_probes(FAILED);
    handle_launch_probes(CONNECTED);
    handle_launch_probes(FAILED);
    handle_launch_probes(FAILED);
    handle_launch_probes(CONNECTED);

    failed.assert();
    successful.assert();
  }

  /// The common case has to keep working exactly as before: a server that comes up and
  /// registers sends one start notification, and no failure.
  #[test]
  #[serial]
  fn a_clean_start_notifies_success_only() {
    let mut webhook = Webhook::new();
    let failed = webhook.expect("Start", "Failed", 0);
    let successful = webhook.expect("Start", "Successful", 1);

    handle_launch_probes(CONNECTED);

    successful.assert();
    failed.assert();
  }

  #[test]
  #[serial]
  fn a_shutdown_still_notifies() {
    let mut webhook = Webhook::new();
    let stopped = webhook.expect("Stop", "Successful", 1);

    handle_launch_probes(DESTROY);

    stopped.assert();
  }

  /// The failure rides on the existing `WEBHOOK_STATUS_FAILED` switch, so anyone who has
  /// turned failure notifications off stays quiet.
  #[test]
  #[serial]
  fn the_failure_respects_the_status_switch() {
    let mut webhook = Webhook::new();
    std::env::set_var("WEBHOOK_STATUS_FAILED", "0");
    let failed = webhook.expect("Start", "Failed", 0);

    handle_launch_probes(FAILED);

    failed.assert();
    assert!(
      REGISTRATION_FAILING.load(Ordering::SeqCst),
      "the streak is still tracked even when the notification is suppressed"
    );
  }
}

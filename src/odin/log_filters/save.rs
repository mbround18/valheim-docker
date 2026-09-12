use crate::notifications::enums::{
  event_status::EventStatus, notification_event::NotificationEvent,
};
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};

const SAVE_FAILED: &str = "Error saving world!";
const SAVE_DONE: &str = "World save (";

/// Whether the last world save failed. Only the first failure of a streak sends a webhook,
/// since the game retries on every save interval.
static SAVE_FAILING: AtomicBool = AtomicBool::new(false);

/// Surfaces failed world saves. Valheim logs `Error saving world! <reason>` and keeps
/// running, so without this the only sign is a stale world after the next restart.
pub fn handle_save_events(line: &str) {
  if let Some(pos) = line.find(SAVE_FAILED) {
    let reason = line[pos + SAVE_FAILED.len()..]
      .split("StackTrace:")
      .next()
      .unwrap_or("")
      .trim();
    error!("World save failed! The server keeps running but progress is not being written to disk: {reason}");
    if !SAVE_FAILING.swap(true, Ordering::SeqCst) {
      NotificationEvent::Save(EventStatus::Failed)
        .send_notification(Some(format!("World save failed: {reason}")));
    }
  } else if line.contains(SAVE_DONE)
    && line.contains("done. Total time")
    && SAVE_FAILING.swap(false, Ordering::SeqCst)
  {
    info!("World save succeeded again.");
    NotificationEvent::Save(EventStatus::Successful)
      .send_notification(Some("World save succeeded again".to_string()));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;

  const FAILED_LINE: &str = "09/11/2026 19:26:13: Error saving world! Access to the path \"/home/steam/.config/unity3d/IronGate/Valheim/worlds_local/Dedicated/00_00__0_3.chunk\" is denied.  StackTrace:   at System.IO.FileStream..ctor (System.String path) [0x0019e] in <297b518d2c9f4ee5bb6b049e9ead4f70>:0 ";
  const DONE_LINE: &str = "09/11/2026 19:21:39: World save (5/5) done. Total time [13ms]";

  #[test]
  #[serial]
  fn failure_sets_the_flag_and_success_clears_it() {
    std::env::remove_var("WEBHOOK_URL");
    SAVE_FAILING.store(false, Ordering::SeqCst);
    handle_save_events(FAILED_LINE);
    assert!(SAVE_FAILING.load(Ordering::SeqCst));
    handle_save_events(FAILED_LINE);
    assert!(SAVE_FAILING.load(Ordering::SeqCst));
    handle_save_events(DONE_LINE);
    assert!(!SAVE_FAILING.load(Ordering::SeqCst));
  }

  #[test]
  #[serial]
  fn unrelated_lines_are_ignored() {
    std::env::remove_var("WEBHOOK_URL");
    SAVE_FAILING.store(false, Ordering::SeqCst);
    handle_save_events("09/11/2026 19:21:39: World save (2/5) Chunks writing done [2ms]");
    handle_save_events("09/11/2026 19:19:46: Game server connected");
    assert!(!SAVE_FAILING.load(Ordering::SeqCst));
  }
}

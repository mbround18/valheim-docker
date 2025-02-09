use crate::notifications::enums::{
  event_status::EventStatus, notification_event::NotificationEvent,
};
use log::debug;

pub fn handle_launch_probes(line: &str) {
  if line.contains("Game server connected") {
    debug!("Detected 'Game server connected'. Sending Start notification.");
    NotificationEvent::Start(EventStatus::Successful).send_notification(None);
  } else if line.contains("Steam manager on destroy") {
    debug!("Detected 'Steam manager on destroy'. Sending Stop notification.");
    NotificationEvent::Stop(EventStatus::Successful).send_notification(None);
  }
}

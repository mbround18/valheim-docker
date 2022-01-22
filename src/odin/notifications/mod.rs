use serde::{Deserialize, Serialize};
use crate::notifications::enums::notification_event::EventType;

pub mod discord;
pub mod enums;

pub const WEBHOOK_URL: &str = "WEBHOOK_URL";
pub const WEBHOOK_INCLUDE_PUBLIC_IP: &str = "WEBHOOK_INCLUDE_PUBLIC_IP";

#[derive(Deserialize, Serialize)]
pub struct NotificationMessage {
  pub(crate) author: String,
  pub(crate) event_type: EventType,
  pub(crate) event_message: String,
  pub(crate) timestamp: String,
}


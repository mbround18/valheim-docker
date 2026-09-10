use crate::notifications::enums::notification_event::{EventType, NotificationEvent};
use crate::notifications::NotificationMessage;
use crate::utils::get_server_name;
use chrono::Local;
use std::env;

use log::{debug, error, info};

pub fn invoke(title: String, message: String, webhook_url: Option<String>) {
  let name = env::var("TITLE").unwrap_or(title);
  let event_message = env::var("MESSAGE").unwrap_or(message);
  let webhook_url = env::var("WEBHOOK_URL").unwrap_or_else(|_| webhook_url.unwrap_or_default());
  let notification = NotificationMessage {
    author: get_server_name(),
    event_type: EventType {
      name,
      status: "Triggered".to_string(),
    },
    event_message,
    timestamp: Local::now().to_rfc3339(),
  };
  if !webhook_url.is_empty() {
    // Keep the confirmation concise: this is an interactive command, so one
    // line is useful feedback, but the full payload is debugging detail.
    info!("Sending Broadcast: {}", notification.event_type.name);
    debug!(
      "Broadcast payload: {}",
      serde_json::to_string_pretty(&notification).unwrap()
    );
    NotificationEvent::Broadcast.send_custom_notification(webhook_url.as_str(), &notification)
  } else {
    error!("Failed to send notification! Webhook url not provided!")
  }
}

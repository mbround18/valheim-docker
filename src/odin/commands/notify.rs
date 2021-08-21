use crate::notifications::enums::notification_event::{EventType, NotificationEvent};
use crate::notifications::NotificationMessage;
use crate::utils::{get_server_name, parse_arg_variable};
use chrono::Local;
use clap::ArgMatches;
use log::{error, info};

pub fn invoke(args: &ArgMatches) {
  let name = String::from(&args.value_of("TITLE").unwrap_or("Broadcast").to_string());
  let event_message = String::from(
    &args
      .value_of("MESSAGE")
      .unwrap_or("Test Notification")
      .to_string(),
  );
  let webhook_url = parse_arg_variable(args, "WEBHOOK_URL", "");
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
    info!(
      "Sending Broadcast: {}",
      serde_json::to_string_pretty(&notification).unwrap()
    );
    NotificationEvent::Broadcast.send_custom_notification(webhook_url.as_str(), &notification)
  } else {
    error!("Failed to send notification! Webhook url not provided!")
  }
}

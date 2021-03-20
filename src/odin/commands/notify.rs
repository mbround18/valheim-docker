use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::parse_arg_variable;
use clap::ArgMatches;
use log::{error, info};

pub fn invoke(args: &ArgMatches) {
  let message = parse_arg_variable(&args, "MESSAGE", String::from("Test Notification"));
  let webhook_url = parse_arg_variable(&args, "webhook_url", "".to_string());
  if !webhook_url.is_empty() {
    info!("Sending Broadcast: {}", message);
    NotificationEvent::Broadcast.send_custom_notification(webhook_url.as_str(), message.as_str())
  } else {
    error!("Failed to send notification! Webhook url not provided!")
  }
}

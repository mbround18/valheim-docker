use std::env;
use std::env::VarError;

use chrono::prelude::*;
use inflections::case::{to_constant_case, to_title_case};
use log::{debug, error, info};
use reqwest::blocking::RequestBuilder;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::notifications::discord::{is_discord_webhook, DiscordWebHookBody};
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::{EventType, NotificationEvent};

mod discord;
pub mod enums;

pub const WEBHOOK_URL: &str = "WEBHOOK_URL";

#[derive(Deserialize, Serialize)]
pub struct NotificationMessage {
  event_type: EventType,
  event_message: String,
  timestamp: String,
}

fn fetch_webhook_url() -> Result<String, VarError> {
  env::var(WEBHOOK_URL)
}

fn is_webhook_enabled() -> bool {
  fetch_webhook_url().is_ok()
}

fn parse_webhook_env_var(event_type: EventType) -> String {
  if event_type.name.to_lowercase().eq("broadcast") {
    to_constant_case(format!("WEBHOOK_{}_MESSAGE", event_type.name).as_str())
  } else {
    to_constant_case(format!("WEBHOOK_{}_{}_MESSAGE", event_type.name, event_type.status).as_str())
  }
}

impl NotificationEvent {
  fn create_notification_message(&self) -> NotificationMessage {
    NotificationMessage {
      event_type: self.to_event_type(),
      event_message: format!(
        "Server Status: {}",
        to_title_case(self.to_string().as_str())
      ),
      timestamp: Local::now().to_rfc3339(),
    }
  }
  fn handle_request(&self, request: RequestBuilder) {
    let response = request.send();
    if let Ok(parsed_response) = response {
      let response_status = parsed_response.status();
      let response_message = parsed_response.text().unwrap();
      match response_status.as_u16() {
        204 | 201 => info!("[{}]: Webhook message sent successfully!", self),
        _ => error!("Request failed! {}, {}", response_status, response_message),
      }
    } else {
      error!(
        "[{}]: Error with webhook! Status {}",
        self,
        response
          .err()
          .unwrap()
          .status()
          .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
          .as_str()
      );
    }
  }
  fn build_request(&self, webhook_url: &str) -> RequestBuilder {
    let client = reqwest::blocking::Client::new();
    debug!("Webhook URL: {}", webhook_url);
    client.post(webhook_url)
  }
  pub fn send_custom_notification(&self, webhook_url: &str, message: &str) {
    let mut notification = self.create_notification_message();
    notification.event_message = message.to_string();
    debug!("Webhook enabled, sending notification {}", self.to_string());

    let mut req = self.build_request(webhook_url);
    req = if is_discord_webhook(webhook_url) {
      info!("Sending discord notification <3");
      req.json(&DiscordWebHookBody::from(&notification))
    } else {
      debug!(
        "Webhook Payload: {}",
        serde_json::to_string(&notification).unwrap()
      );
      req.json(&notification)
    };
    self.handle_request(req);
  }
  pub fn send_notification(&self) {
    if is_webhook_enabled() {
      debug!("Webhook found! Starting notification process...");
      let event = self.create_notification_message();
      let env_var_name = parse_webhook_env_var(event.event_type);
      let notification_message = env::var(env_var_name).unwrap_or(event.event_message);
      self.send_custom_notification(
        fetch_webhook_url().unwrap().replace("\"", "").as_str(),
        notification_message.as_str(),
      );
    } else {
      debug!("Skipping notification, no webhook supplied!");
    }
  }
}

#[cfg(test)]
mod enum_tests {
  use inflections::case::to_title_case;

  use super::*;
  use crate::notifications::enums::event_status::EventStatus;
  use crate::notifications::NotificationEvent::Broadcast;

  #[test]
  fn parse_enum_as_string() {
    assert_eq!(to_title_case(Broadcast.to_string().as_str()), "Broadcast");
  }
  #[test]
  fn parse_enum_create_notification() {
    let event = NotificationEvent::Stop(EventStatus::Running);
    let notification = event.create_notification_message();
    assert_eq!(
      format!(
        "{} {}",
        notification.event_type.name, notification.event_type.status
      ),
      event.to_string()
    );
    assert!(notification.event_message.contains(&event.to_string()));
  }
}

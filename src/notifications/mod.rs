mod discord;

use crate::notifications::discord::{build_discord_payload, is_discord_webhook};
use chrono::prelude::*;
use inflections::case::{to_constant_case, to_title_case};
use log::{debug, error, info};
use reqwest::blocking::RequestBuilder;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::env::VarError;
use std::{env, fmt};

pub const WEBHOOK_URL: &str = "WEBHOOK_URL";

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum NotificationEventType {
  Broadcast,
  //
  Updating,
  UpdateSuccessful,
  UpdateFailed,
  //
  Starting,
  StartSuccessful,
  StartFailed,
  //
  Stopping,
  StopSuccessful,
  StopFailed,
}

#[derive(Deserialize, Serialize)]
pub struct NotificationMessage {
  event_type: NotificationEventType,
  event_message: String,
  timestamp: String,
}

impl fmt::Display for NotificationEventType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

fn fetch_webhook_url() -> Result<String, VarError> {
  env::var(WEBHOOK_URL)
}

fn is_webhook_enabled() -> bool {
  fetch_webhook_url().is_ok()
}

pub trait NotificationEvent {
  fn handle_request(&self, request: RequestBuilder);
  fn create_notification(&self) -> NotificationMessage;
  fn build_request(&self, webhook_url: &str) -> RequestBuilder;
  fn send_custom_notification(&self, webhook_url: &str, message: &str);
  fn send_notification(&self);
}

impl NotificationEvent for NotificationEventType {
  fn handle_request(&self, request: RequestBuilder) {
    let response = request.send();
    if let Ok(parsed_response) = response {
      let response_status = parsed_response.status();
      let response_message = parsed_response.text().unwrap();
      match response_status.as_u16() {
        204 | 201 => info!("[{}]: Webhook message sent successfully!", self.to_string()),
        _ => error!("Request failed! {}, {}", response_status, response_message),
      }
    } else {
      error!(
        "[{}]: Error with webhook! Status {}",
        self.to_string(),
        response
          .err()
          .unwrap()
          .status()
          .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
          .as_str()
      );
    }
  }
  fn create_notification(&self) -> NotificationMessage {
    let name = self.to_string();
    NotificationMessage {
      event_type: self.to_owned(),
      event_message: format!("Server Status: {}", to_title_case(name.as_str())),
      timestamp: Local::now().format("%m/%d/%Y %H:%M:%S %Z").to_string(),
    }
  }
  fn build_request(&self, webhook_url: &str) -> RequestBuilder {
    let client = reqwest::blocking::Client::new();
    client.post(webhook_url)
  }
  fn send_custom_notification(&self, webhook_url: &str, message: &str) {
    let mut notification = self.create_notification();
    notification.event_message = message.to_string();
    debug!("Webhook enabled, sending notification {}", self.to_string());

    let mut req = self.build_request(webhook_url);
    req = if is_discord_webhook(webhook_url) {
      info!("Sending discord notification <3");
      req.json(&build_discord_payload(notification))
    } else {
      debug!(
        "Webhook Payload: {}",
        serde_json::to_string(&notification).unwrap()
      );
      req.json(&notification)
    };
    self.handle_request(req);
  }
  fn send_notification(&self) {
    if is_webhook_enabled() {
      debug!("Webhook found! Starting notification process...");
      let event = self.create_notification();
      let env_var_name = to_constant_case(format!("WEBHOOK_{}_MESSAGE", event.event_type).as_str());
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
  use super::*;
  use inflections::case::to_title_case;

  #[test]
  fn parse_enum_as_string() {
    assert_eq!(
      to_title_case(NotificationEventType::Starting.to_string().as_str()),
      "Starting"
    );
  }
  #[test]
  fn parse_enum_create_notification() {
    let event = NotificationEventType::Stopping;
    let notification = event.create_notification();
    assert_eq!(notification.event_type.to_string(), event.to_string());
    assert!(notification.event_message.contains(&event.to_string()));
  }
}

use crate::errors::VariantNotFound;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::player::PlayerStatus;
use crate::notifications::{
  discord::{is_discord_webhook, DiscordWebHookBody},
  NotificationMessage, WEBHOOK_INCLUDE_PUBLIC_IP, WEBHOOK_URL,
};
use crate::utils::environment::fetch_var;
use crate::utils::{fetch_public_address, get_server_name};
use chrono::Local;
use inflections::case::to_title_case;
use log::{debug, error, info, warn};
use reqwest::{blocking::RequestBuilder, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum NotificationEvent {
  Broadcast,
  Update(EventStatus),
  Start(EventStatus),
  Stop(EventStatus),
  Player(PlayerStatus),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EventType {
  pub(crate) name: String,
  pub(crate) status: String,
}

fn fetch_webhook_url() -> String {
  fetch_var(WEBHOOK_URL, "")
    .trim_start_matches('"')
    .trim_end_matches('"')
    .to_string()
}

fn is_webhook_enabled() -> bool {
  let url = fetch_webhook_url();
  if !url.is_empty() {
    debug!("Webhook Url found!: {}", url);
    let is_valid = Url::parse(url.as_str()).is_ok();
    if !is_valid {
      warn!(
        "Webhook provided but does not look valid!! Is this right? {}",
        url
      )
    }
    return is_valid;
  }
  false
}

fn is_webhook_include_public_ip() -> bool {
  if fetch_var(WEBHOOK_INCLUDE_PUBLIC_IP, "0")
    .trim_start_matches('"')
    .trim_end_matches('"')
    .eq("1")
  {
    debug!("Webhook Include Public IP found!");
    return true;
  }
  false
}

pub fn parse_server_name_for_notification() -> String {
  if is_webhook_include_public_ip() {
    let ip = fetch_public_address();
    format!("{} - {}:{}", get_server_name(), &ip.ip, &ip.port)
  } else {
    get_server_name()
  }
}

impl NotificationEvent {
  fn create_notification_message(&self) -> NotificationMessage {
    NotificationMessage {
      author: format!("Notification: {}", get_server_name()),
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
  pub fn send_custom_notification(&self, webhook_url: &str, notification: &NotificationMessage) {
    debug!("Webhook enabled, sending notification {}", self);
    debug!(
      "Event Received: {}",
      serde_json::to_string_pretty(&notification).unwrap()
    );
    let mut req = self.build_request(webhook_url);
    req = if is_discord_webhook(webhook_url) {
      info!("Sending discord notification <3");
      req.json(&DiscordWebHookBody::from(notification))
    } else {
      debug!(
        "Webhook Payload: {}",
        serde_json::to_string(&notification).unwrap()
      );
      req.json(&notification)
    };
    self.handle_request(req);
  }
  pub fn send_notification(&self, message: Option<String>) {
    debug!("Checking for notification information...");
    if is_webhook_enabled() {
      debug!("Webhook found! Starting notification process...");
      let mut event = self.create_notification_message();
      let enabled_var = format!("WEBHOOK_STATUS_{}", event.event_type.status).to_uppercase();

      if let Some(msg) = message {
        event.event_message = msg;
      }

      debug!("Checking ENV Var: {}", &enabled_var);
      if fetch_var(&enabled_var, "0").eq("1") {
        self.send_custom_notification(&fetch_webhook_url(), &event);
      } else {
        debug!("Skipping notification, {} is set to 0", enabled_var);
      }
    } else {
      debug!("Skipping notification, no webhook supplied!");
    }
  }

  pub(crate) fn to_event_type(&self) -> EventType {
    let event = self.to_string();
    let parsed_event: Vec<&str> = event.split(' ').collect();
    let name = parsed_event.first().unwrap_or(&"EVENT NAME").to_string();
    let status = parsed_event.get(1).unwrap_or(&"Triggered").to_string();
    EventType { name, status }
  }
}

impl fmt::Display for NotificationEvent {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let debug = format!("{:?}", self);
    let formatted = debug.replace('(', " ").replace(')', "");
    f.write_str(&formatted)
  }
}

impl std::str::FromStr for NotificationEvent {
  type Err = VariantNotFound;
  fn from_str(s: &str) -> Result<NotificationEvent, Self::Err> {
    use NotificationEvent::{Broadcast, Player, Start, Stop, Update};
    let parts: Vec<&str> = s.split(' ').collect();
    let event = parts[0];
    if event.eq(Broadcast.to_string().as_str()) {
      Ok(Broadcast)
    } else if event.eq("Player") {
      let player_status = PlayerStatus::from_str(parts[1])?;
      Ok(Player(player_status))
    } else {
      let status = parts[1];
      let event_status = EventStatus::from_str(status)?;
      match event {
        "Update" => Ok(Update(event_status)),
        "Start" => Ok(Start(event_status)),
        "Stop" => Ok(Stop(event_status)),
        _ => Err(VariantNotFound {
          v: String::from("Failed to find Notification Event"),
        }),
      }
    }
  }
}

#[cfg(test)]
mod notification_event_tests {
  use super::*;
  use crate::notifications::enums::player::PlayerStatus;
  use std::str::FromStr;
  use NotificationEvent::{Broadcast, Player};

  #[test]
  fn parse_enum_from_string() {
    assert_eq!(NotificationEvent::from_str("Broadcast").unwrap(), Broadcast);
  }

  #[test]
  fn parse_player_enum_from_string() {
    assert_eq!(
      NotificationEvent::from_str("Player Joined").unwrap(),
      Player(PlayerStatus::Joined)
    );
    assert_eq!(
      NotificationEvent::from_str("Player Left").unwrap(),
      Player(PlayerStatus::Left)
    );
  }
}

#[cfg(test)]
mod webhook_tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  #[test]
  #[serial]
  fn is_webhook_enabled_found_var_valid_url() {
    set_var("WEBHOOK_URL", "http://127.0.0.1:3000/dummy-url");
    assert!(is_webhook_enabled());
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_found_var_invalid_url() {
    set_var("WEBHOOK_URL", "LOCALHOST");
    assert!(!is_webhook_enabled());
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_not_found_var() {
    remove_var("WEBHOOK_URL");
    assert!(!is_webhook_enabled());
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_empty_var() {
    set_var("WEBHOOK_URL", "");
    assert!(!is_webhook_enabled());
  }
}

#[cfg(test)]
mod enum_tests {
  use inflections::case::to_title_case;
  use std::env::set_var;

  use super::*;
  use crate::notifications::enums::event_status::EventStatus;
  use crate::notifications::enums::notification_event::NotificationEvent::Broadcast;

  #[test]
  fn parse_enum_as_string() {
    assert_eq!(to_title_case(Broadcast.to_string().as_str()), "Broadcast");
  }

  #[test]
  fn parse_enum_create_notification() {
    set_var("NAME", "parse_enum_create_notification");
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

  #[test]
  fn parse_player_enum_create_notification() {
    set_var("NAME", "parse_player_enum_create_notification");
    let event = NotificationEvent::Player(PlayerStatus::Joined);
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

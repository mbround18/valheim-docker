use crate::errors::VariantNotFound;
use crate::notifications::enums::event_status::EventStatus;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub enum NotificationEvent {
  Broadcast,
  Update(EventStatus),
  Start(EventStatus),
  Stop(EventStatus),
}

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub struct EventType {
  pub(crate) name: String,
  pub(crate) status: String,
}

impl NotificationEvent {
  pub(crate) fn to_event_type(&self) -> EventType {
    let event = self.to_string();
    let parsed_event: Vec<&str> = event.split(' ').collect();
    let name = parsed_event.get(0).unwrap_or(&"EVENT NAME").to_string();
    let status = parsed_event.get(1).unwrap_or(&"Triggered").to_string();
    EventType { name, status }
  }
}

impl fmt::Display for NotificationEvent {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let debug = format!("{:?}", self);
    let formatted = debug.replace("(", " ").replace(")", "");
    f.write_str(&formatted)
  }
}

impl std::str::FromStr for NotificationEvent {
  type Err = VariantNotFound;
  fn from_str(s: &str) -> core::result::Result<NotificationEvent, Self::Err> {
    use NotificationEvent::{Broadcast, Start, Stop, Update};
    let parts: Vec<&str> = s.split(' ').collect();
    let event = parts[0];
    if event.eq(Broadcast.to_string().as_str()) {
      ::std::result::Result::Ok(Broadcast)
    } else {
      let status = parts[1];
      let event_status = EventStatus::from_str(&status).unwrap();
      match event {
        "Update" => ::std::result::Result::Ok(Update(event_status)),
        "Start" => ::std::result::Result::Ok(Start(event_status)),
        "Stop" => ::std::result::Result::Ok(Stop(event_status)),
        _ => ::std::result::Result::Err(VariantNotFound {
          v: String::from("Failed to find Notification Event"),
        }),
      }
    }
  }
}

#[cfg(test)]
mod notification_event_tests {
  use super::*;
  use std::str::FromStr;
  use NotificationEvent::Broadcast;

  #[test]
  fn parse_enum_from_string() {
    assert_eq!(NotificationEvent::from_str("Broadcast").unwrap(), Broadcast);
  }
}

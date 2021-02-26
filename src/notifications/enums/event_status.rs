use crate::errors::VariantNotFound;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub enum EventStatus {
  Running,
  Successful,
  Failed,
}

impl std::str::FromStr for EventStatus {
  type Err = VariantNotFound;
  fn from_str(s: &str) -> ::std::result::Result<EventStatus, Self::Err> {
    use EventStatus::{Failed, Running, Successful};
    match s {
      "Running" => ::std::result::Result::Ok(Running),
      "Successful" => ::std::result::Result::Ok(Successful),
      "Failed" => ::std::result::Result::Ok(Failed),
      _ => ::std::result::Result::Err(VariantNotFound {
        v: String::from("Failed to find Event Status"),
      }),
    }
  }
}

#[cfg(test)]
mod event_status_tests {
  use super::*;
  use crate::notifications::enums::event_status::EventStatus::Running;
  use std::str::FromStr;

  #[test]
  fn parse_enum_from_string() {
    assert_eq!(Running, EventStatus::from_str("Running").unwrap());
  }
}

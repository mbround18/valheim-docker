use crate::errors::VariantNotFound;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum PlayerStatus {
  Joined,
  Left,
}

impl std::str::FromStr for PlayerStatus {
  type Err = VariantNotFound;
  fn from_str(s: &str) -> Result<PlayerStatus, Self::Err> {
    use PlayerStatus::{Joined, Left};
    match s {
      "Joined" => Ok(Joined),
      "Left" => Ok(Left),
      _ => Err(VariantNotFound {
        v: String::from("Failed to find Player Status"),
      }),
    }
  }
}

#[cfg(test)]
mod player_status_tests {
  use super::*;
  use crate::notifications::enums::player::PlayerStatus::Joined;
  use std::str::FromStr;

  #[test]
  fn parse_enum_from_string() {
    assert_eq!(Joined, PlayerStatus::from_str("Joined").unwrap());
  }
}

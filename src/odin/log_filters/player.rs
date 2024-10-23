use crate::files::FileManager;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::notifications::enums::player::PlayerStatus::{Joined, Left};
use chrono::Utc;
use log::{debug, error, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::num::ParseIntError;

#[derive(Serialize, Deserialize, Debug)]
struct Player {
  id: u64,
  name: String,
  last_seen: i64,
}

impl Clone for Player {
  fn clone(&self) -> Self {
    Player {
      id: self.id,
      name: String::from(&self.name),
      last_seen: self.last_seen,
    }
  }
}

impl Default for Player {
  fn default() -> Self {
    let now = Utc::now();
    let epoch = now.timestamp();

    Player {
      id: 0,
      name: "Unknown".to_string(),
      last_seen: epoch,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerList {
  players: Vec<Player>,
}

impl PlayerList {
  fn save(&self) -> bool {
    self.write(self.to_string())
  }

  fn update_or_push(&mut self, player: Player) {
    if let Some(existing_player) = self.players.iter_mut().find(|p| p.id == player.id) {
      existing_player.last_seen = player.last_seen;
    } else {
      self.players.push(player);
    }
  }

  pub fn joined_event(id: u64, name: String) {
    let mut list = PlayerList::default();
    let now = Utc::now();
    let last_seen = now.timestamp();

    NotificationEvent::Player(Joined)
      .send_notification(Some(format!("Player {name} has joined the adventure!")));

    list.update_or_push(Player {
      id,
      name,
      last_seen,
    });

    list.save();
  }

  pub fn left_event(id: u64) {
    let list = PlayerList::default();
    if let Some(player) = list.players.iter().find(|player| player.id.eq(&id)) {
      let name = &player.name;
      NotificationEvent::Player(Left)
        .send_notification(Some(format!("Player {name} has left the adventure")))
    }
  }
}

impl Default for PlayerList {
  #[cfg(not(test))]
  fn default() -> Self {
    let list = PlayerList { players: vec![] };
    let read_list = list.read();

    if read_list.is_empty() {
      list
    } else {
      PlayerList::from(read_list)
    }
  }

  #[cfg(test)]
  fn default() -> Self {
    PlayerList { players: vec![] }
  }
}

impl FileManager for PlayerList {
  fn path(&self) -> String {
    format!(
      "{}/player.list",
      crate::utils::common_paths::saves_directory()
    )
  }
}

impl From<String> for PlayerList {
  fn from(value: String) -> Self {
    if value.trim().is_empty() {
      PlayerList::default()
    } else {
      serde_json::from_str(&value).expect("Failed to parse player list! Was it modified?")
    }
  }
}

impl Display for PlayerList {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      serde_json::to_string_pretty(&self).expect("Failed to return string of PlayerList")
    )
  }
}

/// Handles player-related events such as joining or leaving.
/// It uses regex to extract information from log lines and triggers appropriate events.
///
/// # Arguments
/// * `line` - A `&str` representing a single line from the log.
pub fn handle_player_events(line: &str) {
  // Regex to capture player joining event with player name and ID
  let joined_regex = Regex::new(
    r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Got character ZDOID from (.*) : (\d+(?::\d+)?)",
  )
  .expect("Failed to compile joined_regex");

  // Regex to capture player leaving event with player ID
  let left_regex = Regex::new(
    r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Destroying abandoned non persistent zdo (\d+:3) owner \d+"
  ).expect("Failed to compile left_regex");

  // Handle player joining event
  if let Some(captures) = joined_regex.captures(line) {
    debug!("Matched joining event: '{:?}'", captures);
    match extract_player_details(&captures) {
      Ok((name, id)) => {
        info!("Player '{}' with ID '{}' is joining", name, id);
        PlayerList::joined_event(id, name);
      }
      Err(e) => error!("Failed to process joining event line '{}': {}", line, e),
    }
  }

  // Handle player leaving event
  if let Some(captures) = left_regex.captures(line) {
    debug!("Matched leaving event: '{:?}'", captures);
    match extract_player_id(captures.get(1).map(|m| m.as_str())) {
      Ok(id) => {
        info!("Player with ID '{}' is leaving", id);
        PlayerList::left_event(id);
      }
      Err(e) => error!("Failed to process leaving event line '{}': {}", line, e),
    }
  }
}

/// Extracts the player name and ID from regex captures for a joining event.
fn extract_player_details(captures: &regex::Captures) -> Result<(String, u64), String> {
  debug!("Extracting player details from captures: '{:?}'", captures);
  let name = captures
    .get(1)
    .ok_or("Missing player name")?
    .as_str()
    .to_string();
  let id_str = captures.get(2).ok_or("Missing player ID")?.as_str();
  match extract_player_id(Some(id_str)) {
    Ok(id) => Ok((name, id)),
    Err(e) => Err(format!("Failed to parse player ID: {}", e)),
  }
}

/// Extracts the player ID from a string, optionally splitting it if it contains a colon.
fn extract_player_id(id_str: Option<&str>) -> Result<u64, String> {
  debug!("Extracting player ID from string: '{:?}'", id_str);
  match id_str {
    Some(id) => id
      .split(':')
      .next()
      .ok_or_else(|| "ID split failed: Missing ':' separator".to_string())?
      .parse::<u64>()
      .map_err(|e: ParseIntError| format!("ID parsing failed: {}", e)),
    None => Err("Player ID not found".to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Utc;
  use mockall::automock;

  #[automock]
  pub trait NotificationEventTrait {
    #[allow(dead_code)]
    fn send_notification(&self, message: Option<String>);
  }

  #[test]
  fn test_update_or_push() {
    let mut player_list = PlayerList { players: vec![] };
    let player = Player {
      id: 1,
      name: "Player1".to_string(),
      last_seen: Utc::now().timestamp(),
    };

    player_list.update_or_push(player.clone());
    assert_eq!(player_list.players.len(), 1);
    assert_eq!(player_list.players[0].id, player.id);
    assert_eq!(player_list.players[0].name, player.name);

    let updated_player = Player {
      id: 1,
      name: "Player1".to_string(),
      last_seen: Utc::now().timestamp() + 100,
    };
    player_list.update_or_push(updated_player.clone());
    assert_eq!(player_list.players.len(), 1);
    assert_eq!(player_list.players[0].last_seen, updated_player.last_seen);
  }

  #[test]
  fn test_joined_event() {
    let id = 1;
    let name = "Player1".to_string();

    PlayerList::joined_event(id, name);
  }

  #[test]
  fn test_left_event() {
    let id = 1;
    let player = Player {
      id,
      name: "Player1".to_string(),
      last_seen: Utc::now().timestamp(),
    };

    let player_list = PlayerList {
      players: vec![player],
    };
    player_list.save();

    PlayerList::left_event(id);
  }

  #[test]
  fn test_player_default() {
    let player = Player::default();
    assert_eq!(player.id, 0);
    assert_eq!(player.name, "Unknown");
  }

  #[test]
  fn test_player_list_default() {
    let player_list = PlayerList::default();
    assert_eq!(player_list.players.len(), 0);
  }

  #[test]
  fn test_player_list_save() {
    let player_list = PlayerList {
      players: vec![Player::default()],
    };
    let result = player_list.save();
    assert!(result);
  }
}

use crate::files::FileManager;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::notifications::enums::player::PlayerStatus::{Joined, Left};
use crate::utils::environment::is_env_var_truthy;
use chrono::Utc;
use log::{debug, error, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug)]
struct Player {
  id: u64,
  zdo_index: u16,
  name: String,
  last_seen: i64,
  #[serde(default)]
  joined_at: i64,
}

impl Clone for Player {
  fn clone(&self) -> Self {
    Player {
      id: self.id,
      zdo_index: self.zdo_index,
      name: String::from(&self.name),
      last_seen: self.last_seen,
      joined_at: self.joined_at,
    }
  }
}

/// A player currently online, as exposed to Huginn.
pub struct OnlinePlayer {
  pub name: String,
  /// Unix timestamp of the join (kept across respawns).
  pub joined_at: i64,
}

impl Default for Player {
  fn default() -> Self {
    let now = Utc::now();
    let epoch = now.timestamp();

    Player {
      id: 0,
      zdo_index: 0,
      name: "Unknown".to_string(),
      last_seen: epoch,
      joined_at: epoch,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerList {
  players: Vec<Player>,
}

impl PlayerList {
  fn save(&self) -> bool {
    self.write(self.to_string()) // Ensure this writes correctly
  }

  /// Records `id` as online under `name`; returns true if the player was not online before.
  fn join(&mut self, id: u64, zdo_index: u16, name: String) -> bool {
    let now = Utc::now().timestamp();
    let existing = self
      .players
      .iter()
      .find(|p| p.id == id)
      .map(|p| p.joined_at);
    let is_new = existing.is_none();
    let joined_at = existing.unwrap_or(now);
    self.players.retain(|p| p.id != id);
    self.players.push(Player {
      id,
      zdo_index,
      name,
      last_seen: now,
      joined_at,
    });
    is_new
  }

  fn leave(&mut self, id: u64) -> Option<Player> {
    let index = self.players.iter().position(|p| p.id == id)?;
    Some(self.players.remove(index))
  }

  pub fn joined_event(id: u64, zdo_index: u16, name: String) {
    let mut list = PlayerList::default();
    if list.join(id, zdo_index, name.clone()) && is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
      NotificationEvent::Player(Joined)
        .send_notification(Some(format!("Player {name} has joined the adventure!")));
    }
    list.save();
  }

  pub fn left_event(id: u64) {
    let mut list = PlayerList::default();
    let Some(player) = list.leave(id) else {
      debug!("No player with ID '{id}' found.");
      return;
    };
    if is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
      let name = &player.name;
      NotificationEvent::Player(Left)
        .send_notification(Some(format!("Player {name} has left the adventure")));
    }
    list.save();
  }

  /// Forgets every player; called when the server starts so the list only reflects this run.
  pub fn clear() {
    PlayerList { players: vec![] }.save();
  }

  /// The players currently online.
  pub fn online() -> Vec<OnlinePlayer> {
    PlayerList::default()
      .players
      .into_iter()
      .map(|p| OnlinePlayer {
        name: p.name,
        joined_at: p.joined_at,
      })
      .collect()
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
  // Regex to capture player joining event with player name, ID and ZDO index
  let joined_regex =
    Regex::new(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Got character ZDOID from (.*) : (\d+:\d+)")
      .expect("Failed to compile joined_regex");

  // Regex to capture player leaving event with the owning player ID
  let left_regex = Regex::new(
    r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Destroying abandoned non persistent zdo \d+:\d+ owner (\d+)"
  ).expect("Failed to compile left_regex");

  // Handle player joining event
  if let Some(captures) = joined_regex.captures(line) {
    debug!("Matched joining event: '{captures:?}'");
    match extract_player_details(&captures) {
      // `0:0` is the character despawning on death, not a join
      Ok((_, 0, _)) => {}
      Ok((name, id, zdo_index)) => {
        info!("Player '{name}' with ID '{id}' and ZDO index '{zdo_index}' is joining");
        PlayerList::joined_event(id, zdo_index, name);
      }
      Err(e) => error!("Failed to process joining event line '{line}': {e}"),
    }
  }

  // Handle player leaving event
  if let Some(captures) = left_regex.captures(line) {
    debug!("Matched leaving event: '{captures:?}'");
    match captures[1].parse::<u64>() {
      Ok(id) => PlayerList::left_event(id),
      Err(e) => error!("Failed to process leaving event line '{line}': {e}"),
    }
  }
}

/// Extracts the player name, ID, and ZDO index from regex captures for a joining event.
fn extract_player_details(captures: &regex::Captures) -> Result<(String, u64, u16), String> {
  debug!("Extracting player details from captures: '{captures:?}'");
  let name = captures
    .get(1)
    .ok_or("Missing player name")?
    .as_str()
    .to_string();
  let id_str = captures
    .get(2)
    .ok_or("Missing player ID and ZDO index")?
    .as_str();
  match extract_player_id_and_zdo_index(Some(id_str)) {
    Ok((id, zdo_index)) => Ok((name, id, zdo_index)),
    Err(e) => Err(format!("Failed to parse player ID and ZDO index: {e}")),
  }
}

/// Extracts the player ID and ZDO index from a string with the format `player_id:zdo_index`.
fn extract_player_id_and_zdo_index(id_str: Option<&str>) -> Result<(u64, u16), String> {
  debug!("Extracting player ID and ZDO index from string: '{id_str:?}'");
  match id_str {
    Some(id) => {
      let parts: Vec<&str> = id.split(':').collect();
      if parts.len() != 2 {
        return Err("ID split failed: Invalid format".to_string());
      }
      let player_id = parts[0]
        .parse::<u64>()
        .map_err(|e| format!("ID parsing failed: {e}"))?;
      let zdo_index = parts[1]
        .parse::<u16>()
        .map_err(|e| format!("ZDO index parsing failed: {e}"))?;
      Ok((player_id, zdo_index))
    }
    None => Err("Player ID and ZDO index not found".to_string()),
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
  fn test_join_and_leave() {
    let mut list = PlayerList { players: vec![] };
    assert!(list.join(1, 1, "Player1".to_string()));
    let joined_at = list.players[0].joined_at;
    // respawn after death: same player, new zdo index, still one entry, no new join
    assert!(!list.join(1, 68, "Player1".to_string()));
    assert_eq!(list.players.len(), 1);
    assert_eq!(list.players[0].zdo_index, 68);
    assert_eq!(list.players[0].joined_at, joined_at);

    assert!(list.leave(2).is_none());
    assert_eq!(list.leave(1).map(|p| p.name).as_deref(), Some("Player1"));
    assert!(list.players.is_empty());
  }

  #[test]
  fn test_joined_event() {
    let id = 1;
    let name = "Player1".to_string();
    PlayerList::joined_event(id, 0, name);
  }

  #[test]
  fn test_left_event() {
    let id = 1;
    let player = Player {
      id,
      zdo_index: 0,
      name: "Player1".to_string(),
      last_seen: Utc::now().timestamp(),
      joined_at: Utc::now().timestamp(),
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

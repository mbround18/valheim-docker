use crate::files::FileManager;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::notifications::enums::player::PlayerStatus::{Joined, Left};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Player {
  id: i64,
  name: String,
  last_seen: i64,
  online: bool,
}

impl Default for Player {
  fn default() -> Self {
    Player {
      id: 0,
      name: "Unknown".to_string(),
      last_seen: Utc::now().timestamp(),
      online: false,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerList {
  players: Vec<Player>,
}

impl PlayerList {
  pub fn new() -> Self {
    let pl = PlayerList { players: vec![] };
    pl.save_player_list();
    pl
  }

  fn update_or_add_player(&mut self, id: i64, name: &str, timestamp: i64) {
    if let Some(player) = self.players.iter_mut().find(|p| p.id == id) {
      if timestamp > player.last_seen {
        player.last_seen = timestamp;
        player.online = true;
      }
    } else {
      self.players.push(Player {
        id,
        name: name.to_string(),
        last_seen: timestamp,
        online: true,
      });
    }
  }

  fn remove_player(&mut self, id: i64) {
    self.players.retain(|p| p.id != id);
  }

  pub fn joined_event(id: i64, name: String) {
    let mut list = Self::load_player_list();
    let now = Utc::now().timestamp();

    list.update_or_add_player(id, &name, now);
    list.save_player_list();
    send_player_notification(Joined, Some(&name));
  }

  pub fn left_event(id: i64) {
    let mut list = Self::load_player_list();
    if let Some(player) = list.players.iter_mut().find(|p| p.id == id) {
      send_player_notification(Left, Some(&player.name));
    }

    list.remove_player(id);

    list.save_player_list();
  }

  pub fn save_player_list(&self) {
    self.save();
  }

  pub fn load_player_list() -> Self {
    Self::default()
  }

  fn save(&self) -> bool {
    self.write(self.to_string())
  }
}

impl Default for PlayerList {
  fn default() -> Self {
    let list = PlayerList { players: vec![] };
    let read_list = list.read();

    if read_list.is_empty() {
      list
    } else {
      PlayerList::from(read_list)
    }
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
      serde_json::from_str(&value).unwrap_or(PlayerList {
        players: Vec::new(),
      })
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

pub enum EventType {
  Joined { id: i64, name: String },
  Left { id: i64 },
}

pub fn match_event(line: &str) -> Option<EventType> {
  let joined_regex = Regex::new(r"Got character ZDOID from (.*) : (-?\d+:\d+)").unwrap();
  let left_regex =
    Regex::new(r"Destroying abandoned non persistent zdo (-?\d+:\d+) owner").unwrap();

  if let Some(captures) = joined_regex.captures(line) {
    // Debugging output to check the captured groups
    println!("Matched joined event: {:?}", captures);
    let name = captures.get(1)?.as_str().to_string();
    let id: i64 = captures.get(2)?.as_str().split(':').next()?.parse().ok()?;
    return Some(EventType::Joined { id, name });
  }

  if let Some(captures) = left_regex.captures(line) {
    // Debugging output to check the captured groups
    println!("Matched left event: {:?}", captures);
    let id: i64 = captures.get(1)?.as_str().split(':').next()?.parse().ok()?;
    return Some(EventType::Left { id });
  }

  None
}

pub fn handle_player_events(line: &str) {
  if let Some(event) = match_event(line) {
    match event {
      EventType::Joined { id, name } => PlayerList::joined_event(id, name),
      EventType::Left { id } => PlayerList::left_event(id),
    }
  }
}

fn send_player_notification(
  event: crate::notifications::enums::player::PlayerStatus,
  name: Option<&str>,
) {
  let message = match event {
    Joined => format!(
      "Player {} has joined the adventure!",
      name.unwrap_or("Unknown")
    ),
    Left => format!(
      "Player {} has left the adventure!",
      name.unwrap_or("Unknown")
    ),
  };
  NotificationEvent::Player(event).send_notification(Some(message));
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_match_event_joined_negative_id() {
    let line = "12/10/2024 14:34:17: Got character ZDOID from Thiccy : -497997179:1";
    if let Some(EventType::Joined { id, name }) = match_event(line) {
      assert_eq!(id, -497997179);
      assert_eq!(name, "Thiccy");
    } else {
      panic!("Failed to match joined event with negative ID");
    }
  }

  #[test]
  fn test_update_or_add_player() {
    let mut list = PlayerList { players: vec![] };
    let id: i64 = 1;
    let name = "Player1".to_string();
    let timestamp = Utc::now().timestamp();

    list.update_or_add_player(id, &name, timestamp);
    assert_eq!(list.players.len(), 1);
    assert_eq!(list.players[0].id, id);
    assert!(list.players[0].online);

    let new_timestamp = timestamp + 10;
    list.update_or_add_player(id, &name, new_timestamp);
    assert_eq!(list.players[0].last_seen, new_timestamp);
  }
}

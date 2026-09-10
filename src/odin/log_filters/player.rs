use crate::files::FileManager;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::notifications::enums::player::PlayerStatus::{Joined, Left};
use crate::utils::environment::is_env_var_truthy;
use chrono::Utc;
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::Path;
use std::sync::LazyLock;

/// `Got character ZDOID from <name> : <peer id>:<zdo index>`; the peer id can be negative.
static JOINED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Got character ZDOID from (.*) : (-?\d+:\d+)")
    .expect("Failed to compile joined_regex")
});

/// `Destroying abandoned non persistent zdo <zdo> owner <peer id>`, logged when a peer's
/// objects are cleaned up after it disconnects.
static LEFT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Destroying abandoned non persistent zdo -?\d+:\d+ owner (-?\d+)",
  )
  .expect("Failed to compile left_regex")
});

#[derive(Serialize, Deserialize, Debug)]
struct Player {
  id: i64,
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
  /// Writes the list atomically. Huginn reads this file from another process on every
  /// `/players` and `/metrics` request, so it is written to a sibling temp file and renamed
  /// over the original: a reader sees the old list or the new one, never half of one.
  fn save(&self) -> bool {
    let path = self.path();
    let tmp = format!("{path}.tmp");
    let result = Path::new(&path)
      .parent()
      .map_or(Ok(()), std::fs::create_dir_all)
      .and_then(|_| std::fs::write(&tmp, self.to_string()))
      .and_then(|_| std::fs::rename(&tmp, &path));
    match result {
      Ok(()) => {
        debug!("Saved player list to {path}");
        true
      }
      Err(e) => {
        error!("Failed to write player list {path}: {e}");
        let _ = std::fs::remove_file(&tmp);
        false
      }
    }
  }

  /// Reads the list from disk. A missing, unreadable or corrupt file is treated as an
  /// empty list rather than a panic, since Huginn calls this while serving requests.
  #[cfg(not(test))]
  fn load() -> Self {
    let empty = PlayerList { players: vec![] };
    match std::fs::read_to_string(empty.path()) {
      Ok(content) => PlayerList::from(content),
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => empty,
      Err(e) => {
        warn!("Could not read player list {}: {e}", empty.path());
        empty
      }
    }
  }

  /// Records `id` as online under `name`; returns true if the player was not online before.
  fn join(&mut self, id: i64, zdo_index: u16, name: String) -> bool {
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

  fn leave(&mut self, id: i64) -> Option<Player> {
    let index = self.players.iter().position(|p| p.id == id)?;
    Some(self.players.remove(index))
  }

  pub fn joined_event(id: i64, zdo_index: u16, name: String) {
    let mut list = PlayerList::default();
    if list.join(id, zdo_index, name.clone()) {
      info!("Player '{name}' joined");
      if is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
        NotificationEvent::Player(Joined)
          .send_notification(Some(format!("Player {name} has joined the adventure!")));
      }
    } else {
      info!("Player '{name}' respawned");
    }
    list.save();
  }

  pub fn left_event(id: i64) {
    let mut list = PlayerList::default();
    let Some(player) = list.leave(id) else {
      debug!("No player with ID '{id}' found.");
      return;
    };
    let name = &player.name;
    info!("Player '{name}' left");
    if is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
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
    PlayerList::load()
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
      return PlayerList { players: vec![] };
    }
    serde_json::from_str(&value).unwrap_or_else(|e| {
      warn!("Ignoring unreadable player list ({e}); treating it as empty");
      PlayerList { players: vec![] }
    })
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

/// A presence change parsed from one server log line.
#[derive(Debug, PartialEq, Eq)]
enum PlayerEvent {
  Joined {
    id: i64,
    zdo_index: u16,
    name: String,
  },
  Left {
    id: i64,
  },
}

/// Parses a server log line into a presence change, if it is one.
fn parse_player_event(line: &str) -> Option<PlayerEvent> {
  if let Some(captures) = JOINED_REGEX.captures(line) {
    debug!("Matched joining event: '{captures:?}'");
    return match extract_player_details(&captures) {
      // `0:0` is the character despawning on death, not a join
      Ok((_, 0, _)) => None,
      Ok((name, id, zdo_index)) => Some(PlayerEvent::Joined {
        id,
        zdo_index,
        name,
      }),
      Err(e) => {
        error!("Failed to process joining event line '{line}': {e}");
        None
      }
    };
  }

  if let Some(captures) = LEFT_REGEX.captures(line) {
    debug!("Matched leaving event: '{captures:?}'");
    return match captures[1].parse::<i64>() {
      Ok(id) => Some(PlayerEvent::Left { id }),
      Err(e) => {
        error!("Failed to process leaving event line '{line}': {e}");
        None
      }
    };
  }

  None
}

/// Handles player-related events such as joining or leaving.
/// It uses regex to extract information from log lines and triggers appropriate events.
///
/// # Arguments
/// * `line` - A `&str` representing a single line from the log.
pub fn handle_player_events(line: &str) {
  match parse_player_event(line) {
    Some(PlayerEvent::Joined {
      id,
      zdo_index,
      name,
    }) => {
      debug!("Player '{name}' with ID '{id}' and ZDO index '{zdo_index}' is joining");
      PlayerList::joined_event(id, zdo_index, name);
    }
    Some(PlayerEvent::Left { id }) => PlayerList::left_event(id),
    None => {}
  }
}

/// Extracts the player name, ID, and ZDO index from regex captures for a joining event.
fn extract_player_details(captures: &regex::Captures) -> Result<(String, i64, u16), String> {
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
fn extract_player_id_and_zdo_index(id_str: Option<&str>) -> Result<(i64, u16), String> {
  debug!("Extracting player ID and ZDO index from string: '{id_str:?}'");
  match id_str {
    Some(id) => {
      let parts: Vec<&str> = id.split(':').collect();
      if parts.len() != 2 {
        return Err("ID split failed: Invalid format".to_string());
      }
      let player_id = parts[0]
        .parse::<i64>()
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
  use serial_test::serial;

  #[automock]
  pub trait NotificationEventTrait {
    #[allow(dead_code)]
    fn send_notification(&self, message: Option<String>);
  }

  /// Points `SAVE_LOCATION` at a temp dir for the test's lifetime so tests never write
  /// `player.list` into the developer's real Valheim save directory.
  struct SaveDir {
    dir: tempfile::TempDir,
  }

  impl SaveDir {
    fn new() -> Self {
      let dir = tempfile::tempdir().expect("tempdir");
      std::env::set_var(crate::constants::SAVE_LOCATION, dir.path());
      SaveDir { dir }
    }

    fn list_path(&self) -> std::path::PathBuf {
      self.dir.path().join("player.list")
    }
  }

  impl Drop for SaveDir {
    fn drop(&mut self) {
      std::env::remove_var(crate::constants::SAVE_LOCATION);
    }
  }

  fn joined(id: i64, zdo_index: u16, name: &str) -> Option<PlayerEvent> {
    Some(PlayerEvent::Joined {
      id,
      zdo_index,
      name: name.to_string(),
    })
  }

  #[test]
  fn negative_session_ids_parse() {
    assert_eq!(
      extract_player_id_and_zdo_index(Some("-1234567890:1")),
      Ok((-1234567890, 1))
    );
    assert!(
      JOINED_REGEX.is_match("09/10/2026 00:00:00: Got character ZDOID from Viking : -1234567890:1")
    );
  }

  /// A full session as it appears in `valheim_server.log`: join, die, respawn, leave.
  #[test]
  fn parses_a_session_from_real_log_lines() {
    let lines = [
      "09/10/2026 18:00:01: Got character ZDOID from Viking : 2130425389:1",
      "09/10/2026 18:05:12: Got character ZDOID from Viking : 0:0",
      "09/10/2026 18:05:20: Got character ZDOID from Viking : 2130425389:68",
      "09/10/2026 18:30:00: Destroying abandoned non persistent zdo 2130425389:1204 owner 2130425389",
    ];
    let events: Vec<_> = lines.iter().map(|l| parse_player_event(l)).collect();
    assert_eq!(
      events,
      [
        joined(2130425389, 1, "Viking"),
        None, // death is not a join
        joined(2130425389, 68, "Viking"),
        Some(PlayerEvent::Left { id: 2130425389 }),
      ]
    );
  }

  #[test]
  fn parses_negative_peer_ids_and_names_with_spaces() {
    assert_eq!(
      parse_player_event("09/10/2026 18:00:01: Got character ZDOID from Sir Lance : -99:3"),
      joined(-99, 3, "Sir Lance")
    );
    assert_eq!(
      parse_player_event(
        "09/10/2026 18:30:00: Destroying abandoned non persistent zdo -99:7 owner -99"
      ),
      Some(PlayerEvent::Left { id: -99 })
    );
  }

  #[test]
  fn ignores_unrelated_lines() {
    for line in [
      "",
      "09/10/2026 18:00:00: Game server connected",
      "09/10/2026 18:00:00: Got character ZDOID from Viking",
      "Got character ZDOID from Viking : 1:1",
    ] {
      assert_eq!(parse_player_event(line), None, "{line:?}");
    }
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

  /// Huginn parses this file while Odin rewrites it; garbage must never panic.
  #[test]
  fn corrupt_or_truncated_lists_read_as_empty() {
    for content in ["not json", "{\"players\": [{\"id\": 1, \"zdo_", "{}", "   "] {
      let list = PlayerList::from(content.to_string());
      assert!(list.players.is_empty(), "{content:?}");
    }
  }

  #[test]
  fn lists_from_before_joined_at_still_parse() {
    let old = r#"{"players":[{"id":42,"zdo_index":1,"name":"Viking","last_seen":1700000000}]}"#;
    let list = PlayerList::from(old.to_string());
    assert_eq!(list.players.len(), 1);
    assert_eq!(list.players[0].joined_at, 0);
  }

  #[test]
  #[serial]
  fn save_replaces_the_file_without_leaving_a_temp_file() {
    let saves = SaveDir::new();
    let mut list = PlayerList { players: vec![] };
    list.join(7, 1, "Viking".to_string());
    assert!(list.save());

    let written = std::fs::read_to_string(saves.list_path()).expect("player.list");
    let reread = PlayerList::from(written);
    assert_eq!(reread.players.len(), 1);
    assert_eq!(reread.players[0].name, "Viking");
    assert!(
      !saves.dir.path().join("player.list.tmp").exists(),
      "temp file should be renamed away"
    );
  }

  #[test]
  #[serial]
  fn test_joined_event() {
    let _saves = SaveDir::new();
    PlayerList::joined_event(1, 0, "Player1".to_string());
  }

  #[test]
  #[serial]
  fn test_left_event() {
    let _saves = SaveDir::new();
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
  #[serial]
  fn test_player_list_save() {
    let _saves = SaveDir::new();
    let player_list = PlayerList {
      players: vec![Player::default()],
    };
    let result = player_list.save();
    assert!(result);
  }
}

use regex::Regex;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::notifications::enums::player::PlayerStatus::Joined;

struct Player {
  
}

struct PlayerList {
  players: Vec<>
}



pub fn player_joined(message: &str) {
  let re = Regex::new(r"<color=orange>(.*?)</color>").unwrap();
  if let Some(captures) = re.captures(message) {
    let name = captures.get(1).map_or("", |m| m.as_str());
    NotificationEvent::Player(Joined).send_notification(Some(format!("Player {name} has joined the server!")));
  }
}

pub fn player_left(message: &str) {
  let re = Regex::new(r"<color=orange>(.*?)</color>").unwrap();

  if let Some(captures) = re.captures(message) {
    let name = captures.get(1).map_or("", |m| m.as_str());
    NotificationEvent::Player(Left).send_notification(Some(format!("Player {name} has left the server!")));
  }
}
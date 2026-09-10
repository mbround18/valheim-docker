use crate::{fetch_info, query_socket_addr};
use odin::log_filters::PlayerList;
use warp::reply::json;
use warp::reply::Json;

#[derive(serde::Serialize)]
pub struct PlayersResponse {
  pub online: bool,
  pub players: u8,
  pub max_players: u8,
  pub names: Vec<String>,
  pub sessions: Vec<Session>,
}

#[derive(serde::Serialize)]
pub struct Session {
  pub name: String,
  pub joined_at: i64,
}

pub fn invoke() -> Json {
  // Reuse cached status information from huginn's fetch path.
  let info = fetch_info();

  // Valheim does not publish names over A2S; Odin tracks them from the server log.
  let sessions: Vec<Session> = PlayerList::online()
    .into_iter()
    .map(|p| Session {
      name: p.name,
      joined_at: p.joined_at,
    })
    .collect();
  let mut names: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();
  if names.is_empty() && info.online {
    let Some(socket) = query_socket_addr() else {
      return json(&PlayersResponse {
        online: info.online,
        players: info.players,
        max_players: info.max_players,
        names,
        sessions,
      });
    };
    if let Ok(client) = a2s::A2SClient::new() {
      match client.players(socket) {
        Ok(players) => {
          names = players
            .into_iter()
            .map(|p| p.name)
            .filter(|n| !n.is_empty())
            .collect();
        }
        Err(_e) => {
          // Ignore errors; names stays empty.
        }
      }
    }
  }

  json(&PlayersResponse {
    online: info.online,
    players: info.players,
    max_players: info.max_players,
    names,
    sessions,
  })
}

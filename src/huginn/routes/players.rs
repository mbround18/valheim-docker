use crate::{fetch_info, query_socket_addr};
use warp::reply::json;
use warp::reply::Json;

#[derive(serde::Serialize)]
pub struct PlayersResponse {
  pub online: bool,
  pub players: u8,
  pub max_players: u8,
  pub names: Vec<String>,
}

pub fn invoke() -> Json {
  // Reuse cached status information from huginn's fetch path.
  let info = fetch_info();

  // Attempt to query player names directly via A2S; degrade gracefully on error.
  let mut names: Vec<String> = Vec::new();
  if info.online {
    let Some(socket) = query_socket_addr() else {
      return json(&PlayersResponse {
        online: info.online,
        players: info.players,
        max_players: info.max_players,
        names,
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
  })
}

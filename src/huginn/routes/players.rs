use odin::utils::environment::fetch_var;
use std::net::SocketAddrV4;
use std::str::FromStr;
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
  let port: u16 = fetch_var("PORT", "2457").parse().unwrap_or(2457);
  let address = fetch_var("ADDRESS", format!("127.0.0.1:{}", port + 1).as_str());
  let socket = SocketAddrV4::from_str(&address).unwrap();

  // Current/max from status query (reuse odin's ServerInfo conversion path)
  let info = odin::server::ServerInfo::from(socket);

  // Attempt to query player names directly via A2S; degrade gracefully on error.
  let mut names: Vec<String> = Vec::new();
  if info.online {
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

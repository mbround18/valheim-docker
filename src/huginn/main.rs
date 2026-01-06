mod routes;

use log::info;
use odin::{server::ServerInfo, utils::environment::fetch_var};
use shared::init_logging_and_tracing;
use std::net::SocketAddrV4;
use std::str::FromStr;
use warp::Filter;

fn fetch_info() -> ServerInfo {
  let port: u16 = fetch_var("PORT", "2457").parse().unwrap();
  let address = fetch_var("ADDRESS", format!("127.0.0.1:{}", port + 1).as_str());
  ServerInfo::from(SocketAddrV4::from_str(&address).unwrap())
}

#[tokio::main]
async fn main() {
  // Logger
  init_logging_and_tracing().expect("Failed to initialize logging and tracing");

  // Routes
  let root = warp::path::end().map(routes::invoke);
  let status = warp::path!("status").map(routes::status::invoke);
  let metrics = warp::path!("metrics").map(routes::metrics::invoke);
  let health = warp::path!("health").map(routes::health::invoke);
  let mods = warp::path!("mods").map(routes::mods::invoke);
  let players = warp::path!("players").map(routes::players::invoke);
  let openapi = warp::path!("openapi.json").map(routes::openapi::invoke);
  let routes = warp::any().and(
    root
      .or(status)
      .or(metrics)
      .or(health)
      .or(mods)
      .or(players)
      .or(openapi),
  );

  // HTTP Server
  let http_port: u16 = fetch_var("HTTP_PORT", "3000").parse().unwrap();

  // Start server
  info!("Starting web server....");
  info!("Navigate to http://127.0.0.1:{http_port}/status to view the server status.");
  warp::serve(routes).run(([0, 0, 0, 0], http_port)).await;
}

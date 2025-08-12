mod routes;

use log::info;
use odin::{logger::initialize_logger, server::ServerInfo, utils::environment::fetch_var};
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
  let debug_mode = fetch_var("DEBUG_MODE", "0").eq("1");
  initialize_logger(debug_mode).unwrap();

  // Routes
  let root = warp::path::end().map(routes::invoke);
  let status = warp::path!("status").map(routes::status::invoke);
  let metrics = warp::path!("metrics").map(routes::metrics::invoke);
  let routes = warp::any().and(root.or(status).or(metrics));

  // HTTP Server
  let http_port: u16 = fetch_var("HTTP_PORT", "3000").parse().unwrap();

  // Start server
  info!("Starting web server....");
  info!("Navigate to http://127.0.0.1:{http_port}/status to view the server status.");
  warp::serve(routes).run(([0, 0, 0, 0], http_port)).await;
}

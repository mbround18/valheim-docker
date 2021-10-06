mod routes;

use crate::routes::{
  connect::{connect, secure_connect},
  metrics::metrics,
  status::status,
};
use log::info;
use odin::{logger::initialize_logger, server::ServerInfo, utils::environment::fetch_var};
use std::net::SocketAddrV4;
use std::str::FromStr;

#[macro_use]
extern crate rocket;

fn fetch_info() -> ServerInfo {
  let port: u16 = fetch_var("PORT", "2457").parse().unwrap();
  let address = fetch_var("ADDRESS", format!("127.0.0.1:{}", port + 1).as_str());
  ServerInfo::from(SocketAddrV4::from_str(&address).unwrap())
}

#[launch]
fn rocket() -> _ {
  let debug_mode = fetch_var("DEBUG_MODE", "0").eq("1");
  let port: u16 = fetch_var("HTTP_PORT", "3000").parse().unwrap();
  let log_level: &str = if debug_mode { "debug" } else { "normal" };
  let figment = rocket::Config::figment()
    .merge(("ident", "Huginn"))
    .merge(("port", &port))
    .merge(("address", "0.0.0.0"))
    .merge(("log_level", log_level));

  initialize_logger(debug_mode).unwrap();

  info!("Starting web server....");
  info!(
    "Navigate to http://127.0.0.1:{}/status to view the server status.",
    &port
  );

  rocket::custom(figment).mount("/", routes![status, metrics, connect, secure_connect])
}

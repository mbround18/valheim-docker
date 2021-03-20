use log::info;
use odin::{logger::initialize_logger, server::ServerInfo, utils::environment::fetch_var};
use warp::{reply::json, Filter};

#[tokio::main]
async fn main() {
  let debug_mode = fetch_var("DEBUG_MODE", "0").eq("1");
  initialize_logger(debug_mode).unwrap();

  let status = warp::path!("status").map(|| {
    let port: u16 = fetch_var("PORT", "2457").parse().unwrap();
    let address = fetch_var("ADDRESS", format!("127.0.0.1:{}", port + 1).as_str());
    let info = ServerInfo::from(address);
    json(&info)
  });
  let http_port: u16 = fetch_var("HTTP_PORT", "3000").parse().unwrap();

  info!("Starting web server on http://127.0.0.1:{}", http_port);
  info!(
    "Navigate to http://127.0.0.1:{}/status to view the server status.",
    http_port
  );
  warp::serve(status).run(([0, 0, 0, 0], http_port)).await;
}

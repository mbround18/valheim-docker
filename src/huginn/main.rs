mod routes;

use log::{error, info, warn};
use odin::{
  constants::GAME_ID,
  installed_mods_with_paths,
  server::{try_get_current_build_id, ServerInfo},
  utils::{environment::fetch_var, steamcmd_args::BetaConfig},
};
use serde::Serialize;
use shared::init_logging_and_tracing;
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use warp::Filter;

#[derive(Clone)]
struct CachedInfo {
  fetched_at: Instant,
  info: ServerInfo,
}

#[derive(Clone)]
struct CachedMods {
  fetched_at: Instant,
  mods: Vec<(String, Option<String>)>, // (name, version)
}

static INFO_CACHE: OnceLock<Mutex<Option<CachedInfo>>> = OnceLock::new();
static MODS_CACHE: OnceLock<Mutex<Option<CachedMods>>> = OnceLock::new();
static HUGINN_STARTED_AT: OnceLock<Instant> = OnceLock::new();

#[derive(Serialize)]
pub(crate) struct MetadataResponse {
  service: ServiceMetadata,
  odin: OdinMetadata,
}

#[derive(Serialize)]
struct ServiceMetadata {
  name: String,
  version: String,
  http_bind: String,
  http_port: u16,
  info_cache_ttl_secs: u64,
  uptime_seconds: u64,
}

#[derive(Serialize)]
struct OdinMetadata {
  game_id: i64,
  game_port: u16,
  query_port: u16,
  query_address: String,
  current_build_id: Option<String>,
  public_server: bool,
  crossplay_enabled: bool,
  validate_on_install: bool,
  staged_updates: bool,
  clean_install: bool,
  beta: BetaMetadata,
  jobs: JobsMetadata,
}

#[derive(Serialize)]
struct BetaMetadata {
  use_public_beta: bool,
  branch: String,
  backwards_compatible_branch: bool,
}

#[derive(Serialize)]
struct JobsMetadata {
  auto_update_enabled: bool,
  auto_update_schedule: String,
  auto_backup_enabled: bool,
  auto_backup_schedule: String,
  scheduled_restart_enabled: bool,
  scheduled_restart_schedule: String,
}

fn parse_u16_env(name: &str, default: u16) -> u16 {
  let raw = fetch_var(name, &default.to_string());
  match raw.parse::<u16>() {
    Ok(v) => v,
    Err(e) => {
      warn!(
        "Invalid {}='{}' ({}). Falling back to {}.",
        name, raw, e, default
      );
      default
    }
  }
}

fn parse_bool_env(name: &str, default: bool) -> bool {
  let raw = fetch_var(name, if default { "1" } else { "0" });
  match raw.trim().to_ascii_lowercase().as_str() {
    "1" | "true" | "yes" | "on" => true,
    "0" | "false" | "no" | "off" => false,
    _ => default,
  }
}

fn cache_ttl() -> Duration {
  Duration::from_secs(parse_u16_env("HUGINN_INFO_CACHE_TTL_SECS", 2) as u64)
}

fn default_query_address() -> String {
  let port = parse_u16_env("PORT", 2456);
  format!("127.0.0.1:{}", port + 1)
}

fn configured_query_address() -> String {
  fetch_var("ADDRESS", &default_query_address())
}

pub(crate) fn query_socket_addr() -> Option<SocketAddrV4> {
  let address = configured_query_address();
  match SocketAddrV4::from_str(&address) {
    Ok(socket) => Some(socket),
    Err(e) => {
      error!(
        "Invalid ADDRESS='{}'. Expected format host:port. Error: {}",
        address, e
      );
      None
    }
  }
}

fn fetch_info() -> ServerInfo {
  let ttl = cache_ttl();
  let cache = INFO_CACHE.get_or_init(|| Mutex::new(None));
  {
    let guard = cache.lock().expect("cache mutex poisoned");
    if let Some(entry) = guard.as_ref() {
      if entry.fetched_at.elapsed() <= ttl {
        return entry.info.clone();
      }
    }
  }

  let fresh = if let Some(socket) = query_socket_addr() {
    ServerInfo::from(socket)
  } else {
    ServerInfo::offline()
  };
  let mut guard = cache.lock().expect("cache mutex poisoned");
  *guard = Some(CachedInfo {
    fetched_at: Instant::now(),
    info: fresh.clone(),
  });
  fresh
}

pub(crate) fn fetch_mods() -> Vec<(String, Option<String>)> {
  let ttl = cache_ttl();
  let cache = MODS_CACHE.get_or_init(|| Mutex::new(None));
  {
    let guard = cache.lock().expect("cache mutex poisoned");
    if let Some(entry) = guard.as_ref() {
      if entry.fetched_at.elapsed() <= ttl {
        return entry.mods.clone();
      }
    }
  }

  let installed = installed_mods_with_paths();
  let fresh: Vec<(String, Option<String>)> = installed
    .into_iter()
    .map(|m| (m.manifest.name, m.manifest.version_number))
    .collect();

  let mut guard = cache.lock().expect("cache mutex poisoned");
  *guard = Some(CachedMods {
    fetched_at: Instant::now(),
    mods: fresh.clone(),
  });
  fresh
}

pub(crate) fn fetch_metadata() -> MetadataResponse {
  let started = HUGINN_STARTED_AT.get_or_init(Instant::now);
  let game_port = parse_u16_env("PORT", 2456);
  let http_port = parse_u16_env("HTTP_PORT", 3000);
  let beta = BetaConfig::from_env();

  MetadataResponse {
    service: ServiceMetadata {
      name: String::from("huginn"),
      version: env!("CARGO_PKG_VERSION").to_string(),
      http_bind: String::from("0.0.0.0"),
      http_port,
      info_cache_ttl_secs: cache_ttl().as_secs(),
      uptime_seconds: started.elapsed().as_secs(),
    },
    odin: OdinMetadata {
      game_id: GAME_ID,
      game_port,
      query_port: game_port.saturating_add(1),
      query_address: configured_query_address(),
      current_build_id: try_get_current_build_id(),
      public_server: parse_bool_env("PUBLIC", false),
      crossplay_enabled: parse_bool_env("ENABLE_CROSSPLAY", false),
      validate_on_install: parse_bool_env("VALIDATE_ON_INSTALL", true),
      staged_updates: parse_bool_env("STAGED_UPDATES", false),
      clean_install: parse_bool_env("CLEAN_INSTALL", false),
      beta: BetaMetadata {
        use_public_beta: beta.use_public_beta,
        branch: beta.branch.clone(),
        backwards_compatible_branch: beta.is_backwards_compatible_branch(),
      },
      jobs: JobsMetadata {
        auto_update_enabled: parse_bool_env("AUTO_UPDATE", false),
        auto_update_schedule: fetch_var("AUTO_UPDATE_SCHEDULE", "0 1 * * *"),
        auto_backup_enabled: parse_bool_env("AUTO_BACKUP", false),
        auto_backup_schedule: fetch_var("AUTO_BACKUP_SCHEDULE", "*/15 * * * *"),
        scheduled_restart_enabled: parse_bool_env("SCHEDULED_RESTART", false),
        scheduled_restart_schedule: fetch_var("SCHEDULED_RESTART_SCHEDULE", "0 2 * * *"),
      },
    },
  }
}

#[tokio::main]
async fn main() {
  // Logger
  init_logging_and_tracing().expect("Failed to initialize logging and tracing");

  // Routes
  let root = warp::path::end().map(routes::invoke);
  let connect_local = warp::path!("connect" / "local")
    .and(warp::header::optional::<String>("sec-fetch-mode"))
    .map(routes::connect::local);
  let connect_remote = warp::path!("connect" / "remote")
    .and(warp::header::optional::<String>("x-forwarded-host"))
    .and(warp::header::optional::<String>("sec-fetch-mode"))
    .map(routes::connect::remote);
  let status = warp::path!("status").map(routes::status::invoke);
  let metrics = warp::path!("metrics").map(routes::metrics::invoke);
  let health = warp::path!("health").map(routes::health::invoke);
  let readiness = warp::path!("readiness").map(routes::readiness::invoke);
  let liveness = warp::path!("liveness").map(routes::liveness::invoke);
  let mods = warp::path!("mods").map(routes::mods::invoke);
  let players = warp::path!("players").map(routes::players::invoke);
  let metadata = warp::path!("metadata").map(routes::metadata::invoke);
  let openapi = warp::path!("openapi.json").map(routes::openapi::invoke);
  let docs = warp::path!("docs").map(routes::docs::invoke);
  let routes = warp::any().and(
    root
      .or(status)
      .or(connect_local)
      .or(connect_remote)
      .or(metrics)
      .or(health)
      .or(readiness)
      .or(liveness)
      .or(mods)
      .or(players)
      .or(metadata)
      .or(openapi)
      .or(docs),
  );

  // HTTP Server
  let http_port = parse_u16_env("HTTP_PORT", 3000);

  // Start server
  info!("Starting web server....");
  info!("Navigate to http://127.0.0.1:{http_port}/status to view the server status.");
  info!("Navigate to http://127.0.0.1:{http_port}/connect/local for local Steam connect URL.");
  info!("Navigate to http://127.0.0.1:{http_port}/connect/remote for remote Steam connect URL.");
  info!("Navigate to http://127.0.0.1:{http_port}/metadata to view safe runtime metadata.");
  info!("Navigate to http://127.0.0.1:{http_port}/docs for API documentation.");
  warp::serve(routes).run(([0, 0, 0, 0], http_port)).await;
}

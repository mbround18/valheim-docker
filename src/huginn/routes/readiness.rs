use crate::fetch_info;
use warp::http::StatusCode;
use warp::reply::{json, with_status, Json, WithStatus};

#[derive(serde::Serialize)]
struct ReadinessResponse {
  ready: bool,
  name: String,
}

/// Kubernetes readiness probe - returns 200 only if server is actually online
pub fn invoke() -> WithStatus<Json> {
  let info = fetch_info();
  let status = if info.online {
    StatusCode::OK
  } else {
    StatusCode::SERVICE_UNAVAILABLE
  };
  let body = ReadinessResponse {
    ready: info.online,
    name: info.name,
  };
  with_status(json(&body), status)
}

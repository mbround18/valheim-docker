use crate::fetch_info;
use warp::http::StatusCode;
use warp::reply::{json, with_status, Json, WithStatus};

#[derive(serde::Serialize)]
struct HealthResponse {
  online: bool,
  name: String,
}

pub fn invoke() -> WithStatus<Json> {
  let info = fetch_info();
  let status = if info.online {
    StatusCode::OK
  } else {
    StatusCode::SERVICE_UNAVAILABLE
  };
  let body = HealthResponse {
    online: info.online,
    name: info.name,
  };
  with_status(json(&body), status)
}

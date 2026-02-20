use warp::http::StatusCode;
use warp::reply::{json, with_status, Json, WithStatus};

#[derive(serde::Serialize)]
struct LivenessResponse {
  alive: bool,
}

/// Kubernetes liveness probe - returns 200 if the service is running
pub fn invoke() -> WithStatus<Json> {
  let body = LivenessResponse { alive: true };
  with_status(json(&body), StatusCode::OK)
}

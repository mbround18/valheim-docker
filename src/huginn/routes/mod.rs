use warp::http::StatusCode;
use warp::reply::{html, with_status, Html, WithStatus};

pub(crate) mod connect;
pub(crate) mod docs;
pub(crate) mod health;
pub(crate) mod liveness;
pub(crate) mod metadata;
pub(crate) mod metrics;
pub(crate) mod mods;
pub(crate) mod openapi;
pub(crate) mod players;
pub(crate) mod readiness;
pub(crate) mod status;

/// Serve the main dashboard page
pub fn invoke() -> WithStatus<Html<String>> {
  let html_content = include_str!("../static/index.html");

  with_status(html(html_content.to_string()), StatusCode::OK)
}

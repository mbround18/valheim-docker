use warp::http::StatusCode;
use warp::reply::{html, with_status, Html, WithStatus};

/// Serve interactive API documentation using Swagger UI
pub fn invoke() -> WithStatus<Html<String>> {
  let html_content = include_str!("../static/swagger.html");

  with_status(html(html_content.to_string()), StatusCode::OK)
}

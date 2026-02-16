use crate::fetch_metadata;
use warp::reply::{json, Json};

pub fn invoke() -> Json {
  json(&fetch_metadata())
}

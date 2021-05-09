use crate::fetch_info;
use warp::reply::{json, Json};

pub fn invoke() -> Json {
  let info = fetch_info();
  json(&info)
}

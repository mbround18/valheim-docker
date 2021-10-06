use crate::fetch_info;
use rocket::response::content::Json;

#[get("/status")]
pub fn status() -> Json<String> {
  Json(serde_json::to_string(&fetch_info()).unwrap())
}

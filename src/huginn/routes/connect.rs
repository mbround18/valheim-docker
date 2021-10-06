use crate::fetch_info;
use rocket::response::Redirect;

#[get("/connect", rank = 1)]
pub fn connect() -> Redirect {
  let connection_url = fetch_info().connection_url;
  log::info!("{}", connection_url);
  Redirect::to(fetch_info().connection_url)
}

#[get("/connect?<password>", rank = 2)]
pub fn secure_connect(password: String) -> Redirect {
  let connection_url = format!("{}/{}", fetch_info().connection_url, password);
  log::info!("{}", connection_url);
  Redirect::to(format!("{}/{}", fetch_info().connection_url, password))
}

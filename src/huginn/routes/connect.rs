#[cfg(feature = "connect")]
#[allow(clippy::declare_interior_mutable_const)]
const STEAM_CONNECT: rocket::http::uri::Absolute<'static> = uri!("steam://connect");

#[cfg(feature = "connect")]
fn get_connection_address() -> String {
  if let Ok(address) = std::env::var("ADDRESS") {
    address
  } else {
    crate::fetch_info()
      .connection_url
      .trim_start_matches("steam://connect/")
      .to_string()
  }
}

#[get("/<ip_address>")]
#[cfg(feature = "connect")]
#[allow(dead_code, unused_variables)]
fn parse_ip(ip_address: String) {}

#[get("/<ip_address>/<password>")]
#[cfg(feature = "connect")]
#[allow(dead_code, unused_variables)]
fn parse_ip_with_password(ip_address: String, password: String) {}

#[cfg(feature = "connect")]
#[get("/connect", rank = 2)]
pub fn connect() -> rocket::response::Redirect {
  rocket::response::Redirect::to(uri!(STEAM_CONNECT, parse_ip(get_connection_address())))
}

#[cfg(feature = "connect")]
#[get("/connect?<password>", rank = 1)]
pub fn secure_connect(password: String) -> Result<rocket::response::Redirect, String> {
  match base64::decode(password) {
    Ok(password_decoded) => {
      let mut password_string = String::from_utf8(password_decoded).unwrap();

      // Get ride of new line if it exists.
      if password_string.ends_with('\n') {
        password_string.pop();
      }

      Ok(rocket::response::Redirect::to(uri!(
        STEAM_CONNECT,
        parse_ip_with_password(get_connection_address(), password_string)
      )))
    }
    Err(_) => {
      log::error!("Failed to decode password parameter!");
      Err(String::from("Failed to decode password parameter!"))
    }
  }
}

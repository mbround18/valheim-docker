use crate::utils::environment::fetch_var;
use std::env;
use std::net::{AddrParseError, SocketAddrV4};
use std::str::FromStr;

// Standardized way of fetching public address
pub fn fetch_public_address() -> Result<SocketAddrV4, AddrParseError> {
  let current_port: u16 = fetch_var("PORT", "2456").parse().unwrap();
  let current_ip = match env::var("ADDRESS") {
    Ok(found_address) => found_address,
    Err(_) => {
      // Make request
      match reqwest::blocking::get("https://api.ipify.org") {
        Ok(result) => String::from(&result.text().unwrap()),
        // Fallback to local IP address
        Err(_) => String::from("127.0.0.1"),
      }
    }
  };
  SocketAddrV4::from_str(&format!("{}:{}", current_ip, current_port + 1))
}

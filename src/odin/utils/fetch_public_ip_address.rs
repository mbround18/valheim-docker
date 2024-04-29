use log::{debug, error};
use reqwest::blocking::Client;
use std::env::VarError;
use std::{env, fmt};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IPResponse {
  ip: String,
}

pub struct IPConfig {
  pub(crate) ip: String,
  pub(crate) port: u16,
}

impl fmt::Display for IPConfig {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}:{}", self.ip, self.port)
  }
}

impl IPConfig {
  fn new(ip: String, port: u16) -> IPConfig {
    IPConfig { ip, port }
  }

  fn default() -> IPConfig {
    IPConfig::new("127.0.0.1".to_string(), 2456)
  }

  fn get_ip_from_env(&self) -> Result<String, VarError> {
    env::var("ADDRESS")
  }

  fn get_port_from_env(&self) -> Result<u16, VarError> {
    env::var("PORT").map(|port| port.parse().unwrap())
  }

  pub fn to_string_from_env(&self) -> Result<IPConfig, VarError> {
    match self.get_ip_from_env() {
      Ok(ip) => match self.get_port_from_env() {
        Ok(port) => {
          if ip.is_empty() {
            error!("IP address is empty");
            Err(VarError::NotPresent)
          } else if port.to_string().is_empty() {
            error!("Port is empty");
            Err(VarError::NotPresent)
          } else {
            Ok(IPConfig::new(ip, port))
          }
        }
        Err(e) => Err(e),
      },
      Err(e) => Err(e),
    }
  }

  pub fn fetch_ip_from_api(&self, client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    let urls = [
      "https://api.ipify.org?format=json",
      "https://api.seeip.org/jsonip?",
      "https://ipinfo.io",
    ];

    for url in urls {
      match client.get(url).send() {
        Ok(response) => match response.json::<IPResponse>() {
          Ok(json) => return Ok(json.ip.to_string()),
          Err(e) => {
            debug!("Failed to parse JSON: {}", e);
            continue;
          }
        },
        Err(e) => {
          debug!("Request failed: {}", e);
          continue;
        }
      }
    }

    Err(Box::new(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      "All IP fetch attempts failed",
    )))
  }
}

// Standardized way of fetching public address
pub fn fetch_public_address() -> IPConfig {
  let client = Client::new();
  let mut ip_config = IPConfig::default();
  debug!("Checking for address in env");
  match ip_config.to_string_from_env() {
    Ok(ip) => {
      debug!("Fetched IP: {}", ip);
      ip
    }
    Err(_) => match ip_config.fetch_ip_from_api(&client) {
      Ok(ip) => {
        debug!("Fetched IP: {}", ip);
        ip_config.ip = ip;
        ip_config
      }
      Err(e) => {
        debug!("Failed to fetch IP: {}", e);
        ip_config
      }
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use lazy_static::lazy_static;
  use std::env;
  use std::sync::Mutex;

  lazy_static! {
    static ref ENV_LOCK: Mutex<()> = Mutex::new(());
  }

  #[test]
  fn test_new() {
    let ip_config = IPConfig::new("192.168.1.1".to_string(), 3000);
    assert_eq!(ip_config.ip, "192.168.1.1");
    assert_eq!(ip_config.port, 3000);
  }

  #[test]
  fn test_default() {
    let ip_config = IPConfig::default();
    assert_eq!(ip_config.ip, "127.0.0.1");
    assert_eq!(ip_config.port, 2456);
  }

  #[test]
  fn test_get_ip_from_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    env::set_var("ADDRESS", "192.168.1.1");

    let ip_config = IPConfig::default();
    let result = ip_config.get_ip_from_env();
    assert_eq!(result.unwrap(), "192.168.1.1");

    env::remove_var("ADDRESS");
  }

  #[test]
  fn test_get_port_from_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    env::set_var("PORT", "3000");

    let ip_config = IPConfig::default();
    let result = ip_config.get_port_from_env();
    assert_eq!(result.unwrap(), 3000);

    env::remove_var("PORT");
  }

  #[test]
  fn test_to_string_from_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    env::set_var("ADDRESS", "192.168.1.1");
    env::set_var("PORT", "3000");

    let ip_config = IPConfig::default();
    let result = ip_config.to_string_from_env().unwrap();
    assert_eq!(result.ip, "192.168.1.1");
    assert_eq!(result.port, 3000);

    env::remove_var("ADDRESS");
    env::remove_var("PORT");
  }

  #[test]
  fn test_fetch_ip_from_api() {
    let client = Client::new();
    let ip_config = IPConfig::default();
    let result = ip_config.fetch_ip_from_api(&client);
    assert!(result.is_ok());
  }

  #[test]
  fn test_display_for_ip_config() {
    let ip_config = IPConfig::new("192.168.1.1".to_string(), 3000);
    assert_eq!(ip_config.to_string(), "192.168.1.1:3000");
  }
}

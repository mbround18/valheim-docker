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

  /// The public-IP lookup services tried in order, first success wins.
  const IP_LOOKUP_URLS: &'static [&'static str] = &[
    "https://api.ipify.org?format=json",
    "https://api.seeip.org/jsonip?",
    "https://ipinfo.io",
  ];

  pub fn fetch_ip_from_api(&self, client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    self.fetch_ip_from(client, Self::IP_LOOKUP_URLS)
  }

  /// The body of [`Self::fetch_ip_from_api`], with the service list as a
  /// parameter so it can be pointed at a local mock server. Without this the
  /// only test of this function called out to three real third-party APIs, so
  /// it failed on any machine without open outbound internet access rather
  /// than when the code was actually wrong.
  fn fetch_ip_from(
    &self,
    client: &Client,
    urls: &[&str],
  ) -> Result<String, Box<dyn std::error::Error>> {
    for url in urls {
      match client.get(*url).send() {
        Ok(response) => match response.json::<IPResponse>() {
          Ok(json) => return Ok(json.ip.clone()),
          Err(e) => {
            debug!("Failed to parse JSON: {e}");
            continue;
          }
        },
        Err(e) => {
          debug!("Request failed: {e}");
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
      debug!("Fetched IP: {ip}");
      ip
    }
    Err(_) => match ip_config.fetch_ip_from_api(&client) {
      Ok(ip) => {
        debug!("Fetched IP: {ip}");
        ip_config.ip = ip;
        ip_config
      }
      Err(e) => {
        debug!("Failed to fetch IP: {e}");
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
    let mut server = mockito::Server::new();
    let mock = server
      .mock("GET", "/")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(r#"{"ip":"203.0.113.7"}"#)
      .create();

    let ip_config = IPConfig::default();
    let result = ip_config.fetch_ip_from(&Client::new(), &[&server.url()]);

    mock.assert();
    assert_eq!(result.unwrap(), "203.0.113.7");
  }

  /// The first service answering with something unparseable must not end the
  /// search — the next one in the list still gets a turn.
  #[test]
  fn test_fetch_ip_falls_through_to_the_next_service() {
    let mut broken = mockito::Server::new();
    let broken_mock = broken
      .mock("GET", "/")
      .with_status(500)
      .with_body("upstream on fire")
      .create();

    let mut working = mockito::Server::new();
    let working_mock = working
      .mock("GET", "/")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(r#"{"ip":"198.51.100.4"}"#)
      .create();

    let ip_config = IPConfig::default();
    let result = ip_config.fetch_ip_from(&Client::new(), &[&broken.url(), &working.url()]);

    broken_mock.assert();
    working_mock.assert();
    assert_eq!(result.unwrap(), "198.51.100.4");
  }

  /// Every service failing is an error, not a silent fallback to the default.
  #[test]
  fn test_fetch_ip_errors_when_every_service_fails() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/").with_status(503).create();

    let ip_config = IPConfig::default();
    let result = ip_config.fetch_ip_from(&Client::new(), &[&server.url()]);

    mock.assert();
    assert!(result.is_err());
  }

  #[test]
  fn test_display_for_ip_config() {
    let ip_config = IPConfig::new("192.168.1.1".to_string(), 3000);
    assert_eq!(ip_config.to_string(), "192.168.1.1:3000");
  }
}

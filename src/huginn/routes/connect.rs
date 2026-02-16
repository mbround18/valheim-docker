use odin::utils::{environment::fetch_var, fetch_public_address};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status, Reply, Response};

#[derive(serde::Serialize)]
struct ConnectUrlResponse {
  steam_url: String,
  host: String,
  port: u16,
  redirect: bool,
}

fn parse_u16_env(name: &str, default: u16) -> u16 {
  fetch_var(name, &default.to_string())
    .parse::<u16>()
    .unwrap_or(default)
}

fn game_port() -> u16 {
  parse_u16_env("PORT", 2456)
}

fn extract_host(value: &str) -> String {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return String::from("127.0.0.1");
  }

  if let Ok(sock) = SocketAddr::from_str(trimmed) {
    return sock.ip().to_string();
  }

  if let Some((host, _)) = trimmed.rsplit_once(':') {
    return host.trim_matches(&['[', ']'][..]).to_string();
  }

  trimmed.trim_matches(&['[', ']'][..]).to_string()
}

fn is_non_routable_host(host: &str) -> bool {
  let lower = host.to_ascii_lowercase();
  if lower == "localhost" {
    return true;
  }

  if let Ok(ip) = IpAddr::from_str(host) {
    return match ip {
      IpAddr::V4(v4) => {
        v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
      }
      IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local(),
    };
  }

  false
}

fn choose_remote_host(host_header: Option<String>) -> String {
  let explicit = fetch_var("CONNECT_REMOTE_HOST", "");
  if !explicit.is_empty() {
    return extract_host(&explicit);
  }

  let public_address = fetch_var("PUBLIC_ADDRESS", "");
  if !public_address.is_empty() {
    let host = extract_host(&public_address);
    if !is_non_routable_host(&host) {
      return host;
    }
  }

  if let Some(host) = host_header {
    let first = host.split(',').next().unwrap_or(&host).trim().to_string();
    let parsed = extract_host(&first);
    if !parsed.is_empty() && !is_non_routable_host(&parsed) {
      return parsed;
    }
  }

  let address = fetch_var("ADDRESS", "");
  if !address.is_empty() {
    let host = extract_host(&address);
    if !is_non_routable_host(&host) {
      return host;
    }
  }

  let fetched = extract_host(&fetch_public_address().to_string());
  if !is_non_routable_host(&fetched) {
    return fetched;
  }

  String::from("127.0.0.1")
}

fn steam_redirect(url: String) -> impl Reply {
  with_header(with_status("", StatusCode::FOUND), "Location", url)
}

fn steam_url(host: &str, port: u16) -> String {
  format!("steam://connect/{host}:{port}")
}

fn should_return_json(sec_fetch_mode: Option<&str>) -> bool {
  sec_fetch_mode
    .map(|m| m.eq_ignore_ascii_case("cors"))
    .unwrap_or(false)
}

fn connect_reply(host: String, port: u16, sec_fetch_mode: Option<String>) -> Response {
  let url = steam_url(&host, port);
  if should_return_json(sec_fetch_mode.as_deref()) {
    return json(&ConnectUrlResponse {
      steam_url: url,
      host,
      port,
      redirect: false,
    })
    .into_response();
  }

  steam_redirect(url).into_response()
}

pub fn local(sec_fetch_mode: Option<String>) -> Response {
  let port = game_port();
  connect_reply(String::from("127.0.0.1"), port, sec_fetch_mode)
}

pub fn remote(host_header: Option<String>, sec_fetch_mode: Option<String>) -> Response {
  let port = game_port();
  connect_reply(choose_remote_host(host_header), port, sec_fetch_mode)
}

#[cfg(test)]
mod tests {
  use super::{extract_host, is_non_routable_host, steam_url};

  #[test]
  fn extract_host_strips_port() {
    assert_eq!(extract_host("example.com:2456"), "example.com");
    assert_eq!(extract_host("1.2.3.4:2456"), "1.2.3.4");
    assert_eq!(extract_host("[::1]:2456"), "::1");
  }

  #[test]
  fn detects_non_routable_hosts() {
    assert!(is_non_routable_host("localhost"));
    assert!(is_non_routable_host("127.0.0.1"));
    assert!(is_non_routable_host("10.1.2.3"));
    assert!(!is_non_routable_host("8.8.8.8"));
    assert!(!is_non_routable_host("example.com"));
  }

  #[test]
  fn steam_url_uses_connect_style() {
    let url = steam_url("127.0.0.1", 2456);
    assert!(url.starts_with("steam://connect/"));
    assert!(url.contains("127.0.0.1:2456"));
  }
}

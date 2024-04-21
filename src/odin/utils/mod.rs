pub mod common_paths;
pub mod environment;
pub mod fetch_public_ip_address;
pub mod is_root;
pub mod parse_truthy;

pub use fetch_public_ip_address::fetch_public_address;

use log::debug;
use std::env;
use std::path::Path;

use crate::constants;
use crate::files::config::{config_file, read_config};
use crate::files::FileManager;
use reqwest::Url;

pub fn get_working_dir() -> String {
  environment::fetch_var(
    constants::ODIN_WORKING_DIR,
    env::current_dir().unwrap().to_str().unwrap(),
  )
}

pub fn get_server_name() -> String {
  // Some contexts currently don't get passed in $NAME so fall back to reading from the config
  // if it's missing or invalid UTF-8
  match env::var("NAME") {
    Ok(name) if !name.is_empty() => name,
    _ => {
      let config_file = config_file();
      debug!(
        "Empty or missing $NAME. Falling back to reading from {}",
        config_file.path()
      );
      let config = read_config(config_file);
      config.name
    }
  }
}

pub fn path_exists(path: &str) -> bool {
  let state = Path::new(path).exists();
  debug!(
    "Path {} {}",
    path,
    if state { "exists" } else { "does not exist" }
  );
  state
}

pub fn parse_file_name(url: &Url, default: &str) -> String {
  String::from(
    url
      .path_segments()
      .and_then(|segments| segments.last())
      .and_then(|name| if name.is_empty() { None } else { Some(name) })
      .unwrap_or(default),
  )
}

pub fn get_md5_hash(context: &str) -> String {
  format!("{:x}", md5::compute(context.as_bytes()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hash_str() {
    assert_eq!(
      get_md5_hash("abcdefghijklmnopqrstuvwxyz"),
      "c3fcd3d76192e4007dfb496cca67e13b"
    );
  }
}

pub fn url_parse_file_type(url: &str) -> String {
  url.split('.').last().unwrap().to_string()
}

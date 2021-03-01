use crate::commands::start::bepinex::{fetch_bepinex_mod_list, is_bepinex_installed};
use crate::utils::{fetch_env, parse_arg_variable};
use a2s::errors::Error;
use a2s::info::Info;
use a2s::A2SClient;
use clap::ArgMatches;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::env;
use std::process::exit;

#[derive(Debug, Deserialize, Serialize)]
struct BepInExStatus {
  installed: bool,
  mods: Vec<String>,
}

impl BepInExStatus {
  fn new() -> BepInExStatus {
    BepInExStatus {
      installed: is_bepinex_installed(),
      mods: fetch_bepinex_mod_list(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
struct ServerStatus {
  name: String,
  version: String,
  players: u8,
  max_players: u8,
  map: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonOutput {
  server: ServerStatus,
  bepinex: BepInExStatus,
}

impl From<Result<Info, Error>> for ServerStatus {
  fn from(info: Result<Info, Error>) -> ServerStatus {
    match info {
      Ok(good_info) => {
        debug!("Map: {}", good_info.map);
        ServerStatus {
          name: good_info.name,
          version: good_info.version,
          players: good_info.players,
          max_players: good_info.max_players,
          map: good_info.map,
        }
      }
      Err(_err) => {
        error!("Failed to request server information!");
        exit(1)
      }
    }
  }
}

fn pretty_print(status: ServerStatus, bepinex: BepInExStatus) {
  info!("Name: {}", status.name);
  info!("Players: {}/{}", status.players, status.max_players);
  info!("Map: {}", status.map);
  if bepinex.installed {
    info!("BepInEx Enabled: {}", bepinex.installed);
    info!("BepInEx Mods: {}", bepinex.mods.join(", "));
  }
}

pub fn invoke(args: &ArgMatches) {
  let output_json = args.is_present("json");
  let has_address = args.is_present("address");
  let parsed_address = if has_address {
    parse_arg_variable(args, "address", "".to_string())
  } else {
    let current_ip = env::var("ADDRESS").unwrap_or_else(|_| {
      reqwest::blocking::get("https://api.ipify.org")
        .unwrap()
        .text()
        .unwrap()
    });
    let current_port: u16 = fetch_env("PORT", "2456", false).parse().unwrap();
    format!("{:?}:{:?}", current_ip, current_port + 1)
  }
  .replace("\"", "")
  .replace("\\", "");
  debug!("Game IP {}", parsed_address);
  let query_client = A2SClient::new().unwrap();
  let server_info = ServerStatus::from(query_client.info(parsed_address));
  let bepinex_info = if has_address {
    BepInExStatus {
      installed: false,
      mods: vec![],
    }
  } else {
    BepInExStatus::new()
  };
  if output_json {
    let output = JsonOutput {
      server: server_info,
      bepinex: bepinex_info,
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
  } else {
    pretty_print(server_info, bepinex_info);
  }
}

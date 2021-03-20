use crate::server::ServerInfo;
use crate::utils::environment::fetch_var;
use crate::utils::parse_arg_variable;
use clap::ArgMatches;
use log::info;
use std::env;

pub fn fetch_public_address() -> String {
  let current_ip = env::var("ADDRESS").unwrap_or_else(|_| {
    reqwest::blocking::get("https://api.ipify.org")
      .unwrap()
      .text()
      .unwrap()
  });
  let current_port: u16 = fetch_var("PORT", "2456").parse().unwrap();
  format!("{:?}:{:?}", current_ip, current_port + 1)
}

fn parse_address(args: &ArgMatches) -> String {
  let has_address = args.is_present("address");
  if has_address {
    parse_arg_variable(args, "address", "".to_string())
  } else {
    fetch_public_address()
  }
}

fn pretty_print(status: ServerInfo) {
  info!("Name: {}", status.name);
  info!("Players: {}/{}", status.players, status.max_players);
  info!("Map: {}", status.map);
}

pub fn invoke(args: &ArgMatches) {
  let output_json = args.is_present("json");
  let address = parse_address(&args);
  let server_info = ServerInfo::from(address);
  if output_json {
    println!("{}", serde_json::to_string_pretty(&server_info).unwrap());
  } else {
    pretty_print(server_info);
  }
}

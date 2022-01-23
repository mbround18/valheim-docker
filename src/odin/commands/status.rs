use crate::server::ServerInfo;
use crate::utils::{fetch_public_address, parse_arg_variable};
use clap::ArgMatches;
use log::{error, info};
use std::env;
use std::net::SocketAddrV4;
use std::process::exit;
use std::str::FromStr;

fn parse_address(address: &str) -> SocketAddrV4 {
  match SocketAddrV4::from_str(address) {
    Ok(parsed_address) => parsed_address,
    Err(_) => {
      error!("Failed to parse supplied address! {}", address);
      exit(1)
    }
  }
}

pub fn invoke(args: &ArgMatches) {
  let output_json = args.is_present("json");
  let use_local = args.is_present("local");
  let address = if use_local {
    String::from("127.0.0.1:2457")
  } else if args.is_present("address") {
    parse_arg_variable(args, "address", "")
  } else {
    match env::var("ADDRESS") {
      Ok(env_address) => env_address,
      Err(_) => fetch_public_address().unwrap().to_string(),
    }
  };
  let parsed_address = parse_address(&address);
  let server_info = ServerInfo::from(parsed_address);
  if output_json {
    println!("{}", serde_json::to_string_pretty(&server_info).unwrap());
  } else {
    info!("{}", server_info)
  }
}

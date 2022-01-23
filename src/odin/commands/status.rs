use crate::server::ServerInfo;
use crate::utils::environment::fetch_var;
use crate::utils::{fetch_public_address, parse_arg_variable};
use clap::ArgMatches;
use log::{error, info};
use std::net::{AddrParseError, SocketAddrV4};
use std::process::exit;
use std::str::FromStr;

fn parse_address(args: &ArgMatches) -> Result<SocketAddrV4, AddrParseError> {
  let has_address = args.is_present("address");
  if has_address {
    SocketAddrV4::from_str(&parse_arg_variable(args, "address", ""))
  } else {
    fetch_public_address()
  }
}

pub fn invoke(args: &ArgMatches) {
  let output_json = args.is_present("json");
  let address = parse_address(args).unwrap_or_else(|_| {
    let addr = fetch_var("ADDRESS", args.value_of("address").unwrap());
    error!("Failed to parse supplied address! {}", addr);
    exit(1)
  });
  let server_info = ServerInfo::from(address);
  if output_json {
    println!("{}", serde_json::to_string_pretty(&server_info).unwrap());
  } else {
    info!("{}", server_info)
  }
}

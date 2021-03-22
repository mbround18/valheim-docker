use crate::server::ServerInfo;
use crate::utils::environment::fetch_var;
use crate::utils::parse_arg_variable;
use clap::ArgMatches;
use log::{error, info};
use std::env;
use std::net::{AddrParseError, SocketAddrV4};
use std::process::exit;
use std::str::FromStr;

pub fn fetch_public_address() -> Result<SocketAddrV4, AddrParseError> {
  let current_ip = env::var("ADDRESS").unwrap_or_else(|_| {
    reqwest::blocking::get("https://api.ipify.org")
      .unwrap()
      .text()
      .unwrap()
  });
  let current_port: u16 = fetch_var("PORT", "2456").parse().unwrap();
  SocketAddrV4::from_str(&format!("{}:{}", current_ip, current_port + 1))
}

fn parse_address(args: &ArgMatches) -> Result<SocketAddrV4, AddrParseError> {
  let has_address = args.is_present("address");
  if has_address {
    SocketAddrV4::from_str(&parse_arg_variable(args, "address", "".to_string()))
  } else {
    fetch_public_address()
  }
}

pub fn invoke(args: &ArgMatches) {
  let output_json = args.is_present("json");
  let address = match parse_address(&args) {
    Ok(addr) => addr.to_string(),
    Err(_) => {
      let addr = fetch_var("ADDRESS", args.value_of("address").unwrap());
      error!("Failed to parse supplied address! {}", addr);
      exit(1)
    }
  };
  let server_info = ServerInfo::from(address);
  if output_json {
    println!("{}", serde_json::to_string_pretty(&server_info).unwrap());
  } else {
    info!("{}", server_info)
  }
}

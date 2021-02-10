use std::env;
use clap::ArgMatches;
use std::process::{exit};
use log::{info,debug};
use std::path::Path;
use sysinfo::{System, Signal, SystemExt, ProcessExt};
use crate::executable::{create_execution};
use std::str::from_utf8;
use std::convert::TryInto;

pub fn get_working_dir() -> String {
    match env::current_dir() {
        Ok(current_dir) => current_dir.display().to_string(),
        _ => {
            println!("Something went wrong!");
            exit(1)
        }
    }
}

fn parse_variable(value: String) -> String {
    return value.trim_start_matches('"').trim_end_matches('"').to_string()
}

pub fn get_variable(args: &ArgMatches, name: &str, default: String) -> String {
    debug!("Checking env for {}", name);
    if let Ok(env_val) = env::var(name.to_uppercase()) {
        if env_val.len() > 0 {
            debug!("Env variable found {}={}", name, env_val);
            return parse_variable(env_val);
        }
    }
    if let Ok(env_val) = env::var(format!("SERVER_{}", name).to_uppercase()) {
        debug!("Env variable found {}={}", name, env_val);
        return parse_variable(env_val);
    }
    parse_variable(args.value_of(name).unwrap_or(default.as_str()).to_string())
}


pub fn server_installed() -> bool {
    Path::new(&[get_working_dir(),  "valheim_server.x86_64".to_string()].join("/")).exists()
}


pub fn send_shutdown() {
    info!("Scanning for Valheim process");
    let pid_scan: &[u8] = &*create_execution("pidof")
        .arg("valheim_server.x86_64").output().unwrap().stdout;

    let pid_str = match from_utf8(pid_scan) {
        Ok(v) => v.replace('\n', ""),
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
    if pid_str.is_empty() {
        info!("Process not found!");
        exit(0)
    }
    info!("Pid Found {}", pid_str);
    let pid: i32 =  pid_str.parse().unwrap();
    let mut system = System::new();
    system.refresh_all();
    let process = system.get_process(pid.try_into().unwrap());

    if let Some(found_process) = process {
        info!("Found Process! Sending Interrupt!");
        found_process.kill(Signal::Interrupt);
    } else {
        info!("Process NOT found!")
    }
}

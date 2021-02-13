use clap::ArgMatches;
use log::{debug, error, info};
use std::env;
use std::path::Path;
use std::process::exit;
use sysinfo::{ProcessExt, Signal, System, SystemExt};

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
    return value
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();
}

pub fn get_variable(args: &ArgMatches, name: &str, default: String) -> String {
    debug!("Checking env for {}", name);
    if let Ok(env_val) = env::var(name.to_uppercase()) {
        if !env_val.is_empty() {
            debug!("Env variable found {}={}", name, env_val);
            return parse_variable(env_val);
        }
    }
    if let Ok(env_val) = env::var(format!("SERVER_{}", name).to_uppercase()) {
        debug!("Env variable found {}={}", name, env_val);
        return parse_variable(env_val);
    }
    parse_variable(
        args.value_of(name)
            .unwrap_or_else(|| default.as_str())
            .to_string(),
    )
}

pub fn server_installed() -> bool {
    Path::new(&[get_working_dir(), "valheim_server.x86_64".to_string()].join("/")).exists()
}

pub fn send_shutdown() {
    info!("Scanning for Valheim process");
    let mut system = System::new();
    system.refresh_all();
    let processes = system.get_process_by_name("valheim_server.x86_64");
    if processes.is_empty() {
        info!("Process NOT found!")
    } else {
        for found_process in processes {
            info!(
                "Found Process with pid {}! Sending Interrupt!",
                found_process.pid()
            );
            if found_process.kill(Signal::Interrupt) {
                info!("Process signal interrupt sent successfully!")
            } else {
                error!("Failed to send signal interrupt!")
            }
        }
    }
}

pub fn wait_for_server_exit() {
    info!("Waiting for server to completely shutdown...");
    let mut system = System::new();
    loop {
        system.refresh_all();
        let processes = system.get_process_by_name("valheim_server.x86_64");
        if processes.is_empty() {
            break;
        }
    }
    info!("Server has been shutdown successfully!")
}

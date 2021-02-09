use std::env;
use clap::ArgMatches;
use std::process::exit;
use log::{debug};
use std::path::Path;

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

use crate::executable::{create_execution, execute_mut};
use std::process::{Stdio};
use clap::ArgMatches;
use std::env;

const SCRIPT_FILE: &str = "/home/steam/valheim/valheim_server.x86_64";

fn get_variable(name: &str, args: Option<&ArgMatches>) -> Option<String> {
    let mut variable_value: Option<String> = None;
    match env::var(name) {
        Ok(val) => variable_value = Option::from(val),
        Err(_e) => {
            if let Some(existing_args) = args {
                match existing_args.value_of(name) {
                    Some(val) => {
                        variable_value = Option::from(val.to_string());
                    }
                    None => {}
                }
            }
        },
    }
    variable_value
}

pub fn invoke(args: Option<&ArgMatches>) {
    let mut command = create_execution(SCRIPT_FILE);
    let updated_command = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("-public 1");
    if let Some(port) = get_variable("PORT", args) {
        println!("Found Port Argument: {}", port);
        updated_command.arg(format!("-port {}", port));
    }
    if let Some(name) = get_variable("NAME", args) {
        println!("Adding Name Argument: {}", name);
        updated_command.arg(format!("-name {}", name));
    }
    if let Some(world) = get_variable("WORLD", args) {
        println!("Adding World Argument: {}", world);
        updated_command.arg(format!("-world {}", world));
    }
    if let Some(password) = get_variable("PASSWORD", args) {
        if password.len() > 0 {
            println!("Adding Password Argument");
            updated_command.arg(format!("-password {}", password));
        }

    }

    execute_mut(updated_command)
}

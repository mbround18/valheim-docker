use crate::executable::{create_execution};
use std::process::{Stdio};
use clap::{ArgMatches};
use crate::utils::{get_variable, server_installed, get_working_dir};
use log::{info, error};
use crate::files::start_server_rusty::{write_rusty_start_script, ValheimArguments};
use crate::files::server_exit;

fn parse_arg(args: &ArgMatches, name: &str, default: &str) -> String {
    format!("-{} \"{}\"", name, get_variable(args, name,default.to_string()))
}

pub fn invoke(args: &ArgMatches) {
    server_exit::delete_if_exist();
    info!("Setting up start scripts...");
    let mut command = create_execution("bash");
    let server_executable = &[get_working_dir(),  "valheim_server.x86_64".to_string()].join("/");
    let script_args = &ValheimArguments {
        port: parse_arg(args, "port", "2456").to_string(),
        name: parse_arg(args, "name", "Valheim Docker"),
        world: parse_arg(args, "world", "Dedicated"),
        password: parse_arg(args, "password", "12345"),
        command: server_executable.to_string()
    };
    let dry_run: bool = args.is_present("dry_run");
    info!("Looking for burial mounds...");
    write_rusty_start_script(script_args, dry_run);
    if !dry_run {
        if server_installed() {
            let updated_command = command
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .arg("-c")
                .arg("./start_server_rusty.sh")
                .env("LD_LIBRARY_PATH", "${PWD}/linux64:${LD_LIBRARY_PATH}");
            match updated_command.output() {
                Ok(output) => print!("Exit with code {}", output.status),
                _ => {
                    error!("An error has occurred!")
                }
            }
        } else {
            error!("Could not find server executable! Please install the server!")
        }
    }
}

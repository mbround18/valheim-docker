use crate::executable::{create_execution, execute_mut, handle_exit_status};
use std::process::{Stdio};
use clap::ArgMatches;
use std::{process};
use crate::utils::{get_working_dir, get_variable};
use std::path::Path;

pub fn invoke(args: Option<&ArgMatches>) {
    let paths = &[get_working_dir(), "valheim_server.x86_64".to_string()];
    let script_path = &paths.join("/");
    let script_file = Path::new(script_path);
    if script_file.exists() {
        let mut command = create_execution(script_path);
        let mut command_arguments: Vec<String> = Vec::new();

        if let Some(port) = get_variable("PORT", args, "2456".to_string()) {
            println!("Found Port Argument: {}", port);
            command_arguments.push(format!("-port \"{}\"", port));
        }
        if let Some(name) = get_variable("NAME", args, "Valheim Docker".to_string()) {
            println!("Adding Name Argument: {}", name);
            command_arguments.push(format!("-name \"{}\"", name));
        }
        if let Some(world) = get_variable("WORLD", args, "Dedicated".to_string()) {
            println!("Adding World Argument: {}", world);
            command_arguments.push(format!("-world \"{}\"", world));
        }
        if let Some(password) = get_variable("PASSWORD", args, "".to_string()) {
            if password.len() > 0 {
                println!("Adding Password Argument");
                command_arguments.push(format!("-password \"{}\"", password));
            }
        }
        println!("{}", command_arguments.join(" "));
        let updated_command = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args(command_arguments)
            .arg("-public 1");

        let result = execute_mut(updated_command);
        handle_exit_status(result, "Server Started Successfully!".to_string());
    } else {
        println!("Cannot start server! valheim_server.x86_64 not found in current directory!");
        process::exit(1)
    }
}

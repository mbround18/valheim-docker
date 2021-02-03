use crate::executable::{create_execution, execute_mut};
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
        let updated_command = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg("-public 1");
        if let Some(port) = get_variable("PORT", args, "2456".to_string()) {
            println!("Found Port Argument: {}", port);
            updated_command.arg(format!("-port {}", port));
        }
        if let Some(name) = get_variable("NAME", args, "Valheim Docker".to_string()) {
            println!("Adding Name Argument: {}", name);
            updated_command.arg(format!("-name {}", name));
        }
        if let Some(world) = get_variable("WORLD", args, "Dedicated".to_string()) {
            println!("Adding World Argument: {}", world);
            updated_command.arg(format!("-world {}", world));
        }
        if let Some(password) = get_variable("PASSWORD", args, "".to_string()) {
            if password.len() > 0 {
                println!("Adding Password Argument");
                updated_command.arg(format!("-password {}", password));
            }

        }
        execute_mut(updated_command);
        println!("Server Started Successfully!");
    } else {
        println!("Cannot start server! valheim_server.x86_64 not found in current directory!");
        process::exit(1)
    }


    // match join_paths(&[get_working_dir(), "valheim_server.x86_64".to_string()].join("/")) {
    //     Ok(server_executable) => {
    //         match server_executable.into_string() {
    //             Ok(script_file) => {
    //
    //             }
    //             _ => {
    //                 println!("Something Went Wrong!");
    //                 process::exit(1)
    //             }
    //         }
    //     },
    //     Err(_e) => {
    //
    //     }
    // }
}

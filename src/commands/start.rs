use crate::executable::{create_execution};
use std::process::{Stdio};
use clap::{ArgMatches};
use std::{process};
use crate::utils::{get_working_dir, get_variable};
use std::path::Path;
use std::fs::{File};
use std::io::Write;


fn create_start_server_script(command: String, arguments: String) {
    let source = &[
        "#!/usr/bin/env bash",
        format!("{} {} &", command.as_str(), arguments.as_str()).as_str(),
        "disown"
    ].join("\n");

    match File::create("./start_server_rusty.sh") {
        Ok(mut file) => {
            match file.write_all(source.as_bytes()) {
                Ok(_) => println!("Successfully written script file."),
                _ => println!("Failed to write script file.")
            };

            match create_execution("chmod").args(&["+x", "./start_server_rusty.sh"]).output() {
                Ok(_) =>println!("Success changing permission"),
                _ => println!("Unable to change permissions")
            };
        }
        _ => println!("Failed to write script file.")
    };
}

pub fn invoke(args: Option<&ArgMatches>) {
    let paths = &[get_working_dir(), "valheim_server.x86_64".to_string()];
    let script_path = &paths.join("/");
    let script_file = Path::new(script_path);
    if script_file.exists() {
        let mut command = create_execution("bash");
        let mut command_arguments: Vec<String> = Vec::new();

        if let Some(port) = get_variable("PORT", args, "2456".to_string()) {
            println!("Found Port Argument: {}", port);
            command_arguments.push(format!("-port {}", port));
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

        create_start_server_script(script_path.to_string(), command_arguments.join(" "));

        let updated_command = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg("-c")
            .arg("./start_server_rusty.sh")
            .env("LD_LIBRARY_PATH", "${PWD}/linux64:${LD_LIBRARY_PATH}");


        match updated_command.output() {
            Ok(output) => print!("Exit with code {}", output.status),
            _ => {
                print!("An error has occurred!")
            }
        }
        // updated_command.exec();
        // let result = execute_mut(updated_command);
        // handle_exit_status(result, "Server Started Successfully!".to_string());

    } else {
        println!("Cannot start server! valheim_server.x86_64 not found in current directory!");
        process::exit(1)
    }
}

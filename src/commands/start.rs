use crate::executable::{create_execution};
use std::process::{Stdio};
use clap::{ArgMatches};
use crate::utils::{get_variable, server_installed, get_working_dir};
use std::fs::{File};
use std::io::Write;
use log::{info, error};
use tinytemplate::TinyTemplate;
use serde::Serialize;

#[derive(Serialize)]
struct Context {
    command: String,
    arguments: String
}

static TEMPLATE: &'static &str = &r#"
#!/usr/bin/env bash
cd "$(dirname "$0")"
# This script will be overwritten at each start!

{command} {arguments}  2>&1 | tee ./output.log  &
disown

"#;

fn parse_to_script(context: Context) -> String{
    let mut tt = TinyTemplate::new();
    tt.add_template(
        "hello", &TEMPLATE).unwrap();
    tt.render("hello", &context).unwrap().replace("&quot;", "\"")
}

fn create_start_server_script(command: String, arguments: String, dry_run: bool) {
    let context = Context {
        command,
        arguments
    };
    let source = parse_to_script(context);
    if dry_run {
        info!("This would have written a file to ./start_server_rusty.sh with content: \n {}", source);
    } else {
        match File::create("./start_server_rusty.sh") {
            Ok(mut file) => {
                match file.write_all(source.as_bytes()) {
                    Ok(_) => println!("Successfully written script file."),
                    _ => println!("Failed to write script file.")
                };
                match create_execution("chmod").args(&["+x", "./start_server_rusty.sh"]).output() {
                    Ok(_) => info!("Success changing permission"),
                    _ => error!("Unable to change permissions")
                };
            }
            _ => error!("Failed to write script file.")
        };
    }
}

fn parse_arg(args: &ArgMatches, name: &str, default: &str) -> String {
    format!("-{} \"{}\"", name, get_variable(args, name,default.to_string()))
}

pub fn invoke(args: &ArgMatches) {
    let mut command = create_execution("bash");
    let command_args: &str = &[
        parse_arg(args, "port", "2456"),
        parse_arg(args, "name", "Valheim Docker"),
        parse_arg(args, "world", "Dedicated"),
        parse_arg(args, "password", "12345"),
    ].join(" ");
    let dry_run: bool = args.is_present("dry_run");
    let server_executable = &[get_working_dir(),  "valheim_server.x86_64".to_string()].join("/");
    create_start_server_script(server_executable.to_string(), command_args.to_string(), dry_run);
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

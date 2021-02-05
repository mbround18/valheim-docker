use crate::utils::{get_working_dir, server_installed};
use crate::executable::{create_execution, execute_mut, handle_exit_status};
use std::process::Stdio;
use log::{info, error};
use clap::ArgMatches;

pub fn invoke(args: &ArgMatches) {
    let paths = &[get_working_dir(), "server_exit.drp".to_string()];
    let script_path = &paths.join("/");
    let mut command = create_execution("echo");
    info!("Stopping server");
    let command_arguments = format!("> {}", script_path);
    if args.is_present("dry_run") {
        info!("This command would have run: ");
        info!("echo {}", command_arguments)
    } else {
        if !server_installed() {
            error!("Failed to find server executable!");
            return;
        }
        let updated_command = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg(command_arguments);
        let result = execute_mut(updated_command);
        handle_exit_status(result, "Server Stopped Successfully!".to_string())
    }
}

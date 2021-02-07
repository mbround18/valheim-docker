use crate::utils::{get_working_dir, server_installed};
use log::{info, error, debug};
use clap::ArgMatches;
use crate::files::server_pid::is_running;
use std::thread::sleep;
use std::time::Duration;
use crate::files::server_exit::stop_server;

pub fn invoke(args: &ArgMatches) {
    let paths = &[get_working_dir(), "server_exit.drp".to_string()];
    let script_path = &paths.join("/");
    info!("Stopping server {}", get_working_dir());
    let command_arguments = format!("> {}", script_path);
    if args.is_present("dry_run") {
        info!("This command would have run: ");
        info!("echo {}", command_arguments)
    } else {
        if !server_installed() {
            error!("Failed to find server executable!");
            return;
        }
        stop_server();
        info!("Server Stop triggered! Waiting for server to stop.");
        loop {
            if is_running() {
                debug!("Server is still running...");
                sleep(Duration::from_secs(2));
            } else {
                info!("Server has been stopped!");
                break;
            }
        }
    }
}

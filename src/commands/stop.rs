use crate::utils::{get_working_dir, server_installed, send_shutdown, wait_for_server_exit};
use log::{info, error};
use clap::ArgMatches;

pub fn invoke(args: &ArgMatches) {
    info!("Stopping server {}", get_working_dir());
    if args.is_present("dry_run") {
        info!("This command would have run: ");
        info!("kill -2 valheim_server.x86_64")
    } else {
        if !server_installed() {
            error!("Failed to find server executable!");
            return;
        }
        send_shutdown();
        wait_for_server_exit();
    }
}

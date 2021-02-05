use crate::utils::{get_working_dir, server_installed};
use log::{info, error};
use clap::ArgMatches;
use std::fs::{File};
use std::io::Write;

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
        let mut file = File::create(script_path).unwrap();
        file.write_all(b"1").unwrap();
        info!("Stop file created, Check logs, server should be stopping.");
    }
}

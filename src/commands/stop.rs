use crate::utils::{get_working_dir, server_installed, VALHEIM_EXECUTABLE_NAME};

use clap::ArgMatches;
use log::{error, info};
use sysinfo::{ProcessExt, Signal, System, SystemExt};

use std::{thread, time::Duration};

fn send_shutdown() {
    info!("Scanning for Valheim process");
    let mut system = System::new();
    system.refresh_all();
    let processes = system.get_process_by_name(VALHEIM_EXECUTABLE_NAME);
    if processes.is_empty() {
        info!("Process NOT found!")
    } else {
        for found_process in processes {
            info!(
                "Found Process with pid {}! Sending Interrupt!",
                found_process.pid()
            );
            if found_process.kill(Signal::Interrupt) {
                info!("Process signal interrupt sent successfully!")
            } else {
                error!("Failed to send signal interrupt!")
            }
        }
    }
}

fn wait_for_server_exit() {
    info!("Waiting for server to completely shutdown...");
    let mut system = System::new();
    loop {
        system.refresh_all();
        let processes = system.get_process_by_name(VALHEIM_EXECUTABLE_NAME);
        if processes.is_empty() {
            break;
        } else {
            // Delay to keep down CPU usage
            thread::sleep(Duration::from_secs(1));
        }
    }
    info!("Server has been shutdown successfully!")
}

pub fn invoke(args: &ArgMatches) {
    info!("Stopping server {}", get_working_dir());
    if args.is_present("dry_run") {
        info!("This command would have run: ");
        info!("kill -2 {}", VALHEIM_EXECUTABLE_NAME)
    } else {
        if !server_installed() {
            error!("Failed to find server executable!");
            return;
        }
        send_shutdown();
        wait_for_server_exit();
    }
}

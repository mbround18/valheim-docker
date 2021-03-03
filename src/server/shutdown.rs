use log::{error, info};
use sysinfo::{ProcessExt, Signal, System, SystemExt};

use std::{thread, time::Duration};

use crate::constants;

pub fn blocking_shutdown() {
  send_shutdown_signal();
  wait_for_exit();
}

pub fn send_shutdown_signal() {
  info!("Scanning for Valheim process");
  let mut system = System::new();
  system.refresh_all();
  let processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);
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

pub fn wait_for_exit() {
  info!("Waiting for server to completely shutdown...");
  let mut system = System::new();
  loop {
    system.refresh_all();
    let processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);
    if processes.is_empty() {
      break;
    } else {
      // Delay to keep down CPU usage
      thread::sleep(Duration::from_secs(1));
    }
  }
  info!("Server has been shutdown successfully!")
}

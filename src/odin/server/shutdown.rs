use log::{error, info};
use sysinfo::{ProcessExt, Signal};

use std::{thread, time::Duration};

use super::process::ServerProcess;

pub fn blocking_shutdown() {
  send_shutdown_signal();
  wait_for_exit();
}

pub fn send_shutdown_signal() {
  info!("Scanning for Valheim process");
  let mut server_process = ServerProcess::new();
  let processes = server_process.get_valheim_processes();
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

fn wait_for_exit() {
  info!("Waiting for server to completely shutdown...");
  let mut server_process = ServerProcess::new();
  loop {
    if server_process.is_running() {
      break;
    } else {
      // Delay to keep down CPU usage
      thread::sleep(Duration::from_secs(1));
    }
  }
  info!("Server has been shutdown successfully!")
}

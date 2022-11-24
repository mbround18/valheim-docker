use log::debug;

use std::{thread, time::Duration};

use crate::server::process::ServerProcess;

pub fn blocking_shutdown() {
  let mut server_process = ServerProcess::new();
  server_process.send_interrupt();
  thread::sleep(Duration::from_secs(5));
  loop {
    let mut server = server_process.clone();
    debug!("Checking if valheim is still running.");
    if !server.are_process_running() {
      debug!("Valheim process has been stopped successfully!");
      break;
    } else {
      debug!("Sleeping for 5s to wait for process to stop.");
      thread::sleep(Duration::from_secs(5));
    }
  }
}

use crate::constants;
use log::{debug, error, info};
use sysinfo::{ProcessExt, Signal, System, SystemExt};

pub struct ServerProcess {
  system: System,
}

impl Clone for ServerProcess {
  fn clone(&self) -> Self {
    ServerProcess::new()
  }
}

impl ServerProcess {
  pub fn new() -> ServerProcess {
    ServerProcess {
      system: System::new_all(),
    }
  }

  pub fn valheim_processes(&mut self) -> Vec<&sysinfo::Process> {
    let mut processes = Vec::new();

    self.system.refresh_processes();
    debug!(
      "Scanning for Valheim process via system module. Num of processes: {}",
      self.system.processes().len()
    );

    // Limit search string to 15 characters, as some unix operating systems
    // cannot handle more then 15 character long process names
    for (pid, process) in self.system.processes() {
      debug!("Looking at: PID: {}; Process: {:?}", pid, process.exe());

      if process
        .exe()
        .to_str()
        .unwrap()
        .contains(constants::VALHEIM_EXECUTABLE_NAME)
      {
        debug!("Found process with name: {}", process.name());
        processes.push(process);
      } else {
        continue;
      }
    }

    processes
  }

  pub fn are_process_running(&mut self) -> bool {
    !self.valheim_processes().is_empty()
  }

  pub fn send_interrupt(&mut self) {
    info!("Scanning for Valheim process");
    let processes = self.valheim_processes();
    if processes.is_empty() {
      info!("Process NOT found!")
    } else {
      for found_process in processes {
        info!(
          "Found Process with pid {}! Sending Interrupt!",
          found_process.pid()
        );
        if found_process.kill_with(Signal::Interrupt).unwrap() {
          info!("Process signal interrupt sent successfully!")
        } else {
          error!("Failed to send signal interrupt!")
        }
      }
    }
  }
}

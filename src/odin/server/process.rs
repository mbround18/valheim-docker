use crate::constants;
use log::{debug, error, info};
use std::collections::HashSet;
use sysinfo::{Pid, Signal, System};

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
    self.system.refresh_all();
    debug!(
      "Scanning for Valheim processes via system module. Number of processes: {}",
      self.system.processes().len()
    );

    self
      .system
      .processes()
      .values()
      .filter(|process| is_valheim_executable(process))
      .collect()
  }

  pub fn are_process_running(&mut self) -> bool {
    !self.valheim_processes().is_empty()
  }

  pub fn send_interrupt_to_pid(pid: u32) {
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
      info!("Found process with PID: {pid}");
      match process.kill_with(Signal::Interrupt) {
        Some(_) => info!("Sent interrupt signal to PID: {pid}"),
        None => error!("Failed to send interrupt signal to PID: {pid}."),
      };
    } else {
      debug!("[{pid}]: failed to find process with PID... This can be good and means we stopped it successfully.");
    }
  }

  pub fn send_interrupt(&mut self) {
    let processes = self.valheim_processes();
    // Linux lists each thread of the server as its own entry, parented to the main process.
    // Signal only the entries whose parent is not itself a Valheim entry from this same scan.
    // Re-querying the parent instead races with a server that is already exiting: its
    // executable can no longer be read, which is what used to panic here (#1543).
    let valheim_pids: HashSet<Pid> = processes.iter().map(|process| process.pid()).collect();
    for process in processes {
      if process
        .parent()
        .is_some_and(|parent| valheim_pids.contains(&parent))
      {
        continue;
      }
      let pid = process.pid();
      info!("Found Valheim process with PID: {}", pid.as_u32());
      ServerProcess::send_interrupt_to_pid(pid.as_u32());
    }
  }
}

fn is_valheim_executable(process: &sysinfo::Process) -> bool {
  process.exe().is_some_and(|exe| {
    exe
      .to_string_lossy()
      .contains(constants::VALHEIM_EXECUTABLE_NAME)
  })
}

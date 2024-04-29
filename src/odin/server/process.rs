use crate::constants;
use log::{debug, error, info};
use std::option::Option;
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
    self.system.refresh_processes();
    debug!(
      "Scanning for Valheim processes via system module. Number of processes: {}",
      self.system.processes().len()
    );

    self
      .system
      .processes()
      .values()
      .filter(|process| {
        let path = process.exe().unwrap().to_str().unwrap();
        path.contains(constants::VALHEIM_EXECUTABLE_NAME)
      })
      .collect()
  }

  pub fn get_parent_process(process: &sysinfo::Process) -> Option<Pid> {
    System::new_all()
      .process(process.parent().unwrap())
      .map(|parent| parent.pid())
  }

  pub fn are_process_running(&mut self) -> bool {
    !self.valheim_processes().is_empty()
  }

  pub fn send_interrupt_to_pid(pid: u32) {
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
      info!("Found process with PID: {}", pid);
      match process.kill_with(Signal::Interrupt) {
        Some(_) => info!("Sent interrupt signal to PID: {}", pid),
        None => error!("Failed to send interrupt signal to PID: {}.", pid),
      };
    } else {
      debug!("[{}]: failed to find process with PID... This can be good and means we stopped it successfully.", pid);
    }
  }

  pub fn send_interrupt(&mut self) {
    let processes = self.valheim_processes();
    for process in processes {
      if let Some(parent) = ServerProcess::get_parent_process(process) {
        let s = System::new_all();
        if !s
          .process(parent)
          .unwrap()
          .exe()
          .unwrap()
          .to_str()
          .unwrap()
          .contains(constants::VALHEIM_EXECUTABLE_NAME)
        {
          let pid = process.pid();
          info!("Found Valheim process with PID: {}", pid.as_u32());
          ServerProcess::send_interrupt_to_pid(pid.as_u32());
        }
      }
    }
  }
}

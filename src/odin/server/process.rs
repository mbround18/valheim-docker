use crate::constants;
use sysinfo::{System, SystemExt};

pub struct ServerProcess {
  system: System,
}

impl ServerProcess {
  pub fn new() -> ServerProcess {
    ServerProcess {
      system: System::new_all(),
    }
  }

  pub fn valheim_processes(&mut self) -> Vec<&sysinfo::Process> {
    self.system.refresh_processes();
    // Limit search string to 15 characters, as some unix operating systems
    // cannot handle more then 15 character long process names
    self
      .system
      .process_by_name(&constants::VALHEIM_EXECUTABLE_NAME[..15])
  }

  pub fn are_process_running(&mut self) -> bool {
    !self.valheim_processes().is_empty()
  }
}

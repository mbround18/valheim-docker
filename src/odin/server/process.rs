use sysinfo::{Process, System, SystemExt};

use crate::constants;

pub struct ServerProcess {
  system: System,
}

impl ServerProcess {
  pub fn new() -> ServerProcess {
    ServerProcess {
      system: System::new(),
    }
  }

  pub fn get_valheim_processes(&mut self) -> Vec<&Process> {
    self.system.refresh_processes();

    // Limit search string to 15 characters, as some unix operating systems
    // cannot handle more then 15 character long process names
    let valheim_executable_search_name = &constants::VALHEIM_EXECUTABLE_NAME[..15];

    self
      .system
      .get_process_by_name(valheim_executable_search_name)
  }

  pub fn is_running(&mut self) -> bool {
    let valheim_processes = self.get_valheim_processes();
    !valheim_processes.is_empty()
  }
}

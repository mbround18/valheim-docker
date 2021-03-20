use sysinfo::{System, SystemExt};

use crate::constants;

pub fn is_running() -> bool {
  let mut system = System::new();
  system.refresh_processes();
  let valheim_processes = system.get_process_by_name(constants::VALHEIM_EXECUTABLE_NAME);

  !valheim_processes.is_empty()
}

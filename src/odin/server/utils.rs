use super::process::ServerProcess;

pub fn is_running() -> bool {
  ServerProcess::new().are_process_running()
}

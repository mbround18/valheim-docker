use super::process::ServerProcess;

pub fn is_running() -> bool {
  ServerProcess::new().is_running()
}

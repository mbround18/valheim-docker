use crate::server;
use std::process::ExitStatus;

pub fn invoke(app_id: i64) -> std::io::Result<ExitStatus> {
  server::install(app_id)
}

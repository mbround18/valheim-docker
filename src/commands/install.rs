use crate::executable::execute_mut;
use crate::steamcmd::steamcmd_command;
use crate::utils::get_working_dir;
use log::{debug, info};
use std::process::{ExitStatus, Stdio};

pub fn invoke(app_id: i64) -> std::io::Result<ExitStatus> {
  info!("Installing {} to {}", app_id, get_working_dir());
  let login = "+login anonymous".to_string();
  debug!("Argument set: {}", login);
  let force_install_dir = format!("+force_install_dir {}", get_working_dir());
  debug!("Argument set: {}", force_install_dir);
  let app_update = format!("+app_update {}", app_id);
  debug!("Argument set: {}", app_update);
  let mut steamcmd = steamcmd_command();
  debug!("Setting up install command...");
  let install_command = steamcmd
    .args(&[login, force_install_dir, app_update])
    .arg("+quit")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
  debug!("Launching up install command...");
  execute_mut(install_command)
}

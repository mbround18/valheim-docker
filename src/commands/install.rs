use crate::executable::execute_mut;
use crate::steamcmd::steamcmd_command;
use crate::utils::get_working_dir;
use log::info;
use std::process::{ExitStatus, Stdio};

pub fn invoke(app_id: i64) -> std::io::Result<ExitStatus> {
    info!("Installing {} to {}", app_id, get_working_dir());
    let login = "+login anonymous".to_string();
    let force_install_dir = format!("+force_install_dir {}", get_working_dir());
    let app_update = format!("+app_update {}", app_id);
    let mut steamcmd = steamcmd_command();
    let install_command = steamcmd
        .args(&[login, force_install_dir, app_update])
        .arg("+quit")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    execute_mut(install_command)
}

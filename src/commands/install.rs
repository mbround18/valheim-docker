use crate::steamcmd::steamcmd_command;
use crate::executable::{execute_mut};
use std::process::Stdio;

pub fn invoke(app_id: i64, install_dir: &str) {
    println!("Installing {} to {}", app_id, install_dir);
    let login = "+login anonymous".to_string();
    let force_install_dir = format!("+force_install_dir {}", install_dir).to_string();
    let app_update = format!("+app_update {}", app_id);
    let mut steamcmd =  steamcmd_command();
    let install_command = steamcmd
        .args(&[login, force_install_dir, app_update])
        .arg("+quit")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    execute_mut(install_command)
}

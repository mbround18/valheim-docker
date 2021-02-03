use crate::utils::get_working_dir;
use crate::executable::{create_execution, execute_mut, handle_exit_status};
use std::process::Stdio;

pub fn invoke() {
    let paths = &[get_working_dir(), "server_exit.drp".to_string()];
    let script_path = &paths.join("/");
    let mut command = create_execution(format!("echo 1 > {}", script_path).as_str());
    let updated_command = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let result = execute_mut(updated_command);
    handle_exit_status(result, "Server Stopped Successfully!".to_string())

}

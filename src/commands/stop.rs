use crate::utils::get_working_dir;
use crate::executable::{create_execution, execute_mut};
use std::process::Stdio;
use std::process;
use std::path::Path;

pub fn invoke() {
    let paths = &[get_working_dir(), "server_exit.drp".to_string()];
    let script_path = &paths.join("/");
    let script_file = Path::new(script_path);
    if script_file.exists() {
        let mut command = create_execution(format!("echo 1 > {}", script_path).as_str());
        let updated_command = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        execute_mut(updated_command);
        println!("Server Stopped Successfully!");
    } else {
        println!("Cannot stop server! server_exit.drp not found in current directory!");
        process::exit(1)
    }
}

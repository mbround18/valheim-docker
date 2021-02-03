use std::process::{Command};

pub fn create_execution(script_path: &str) -> Command {
    println!("Executing: {} .....", script_path.to_string());
    Command::new(script_path.to_string())
}

pub fn execute_mut(command: &mut Command) {
    let status =  command
        .spawn()
        .unwrap()
        .wait();
    println!("Exited with status {:?}", status);
}

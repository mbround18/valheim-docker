use std::process::{Command, exit, ExitStatus};
use std::path::Path;


pub fn find_command(executable: &str) -> Option<Command> {
    let script_file = Path::new(executable);
    if script_file.exists() {
        println!("Executing: {} .....", executable.to_string());
        Option::from(Command::new(executable.to_string()))
    } else {
        match which::which(executable) {
            Ok(executable_path) => {
                Option::from(Command::new(executable_path))
            },
            Err(_e) => {
                eprint!("Failed to find {}", executable);
                None
            }
        }
    }
}

pub fn create_execution(executable: &str) -> Command {
    match find_command(executable) {
        Some(command) => command,
        None => {
            eprint!("Unable to launch command {}", executable);
            exit(1)
        }
    }
}

pub fn execute_mut(command: &mut Command) -> std::io::Result<ExitStatus> {
    match command.spawn() {
        Ok(mut subprocess) => subprocess.wait(),
        _ => {
            println!("Failed to run process!");
            exit(1)
        }
    }
}

pub fn handle_exit_status(result: std::io::Result<ExitStatus>, success_message: String) {
    match result {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("{}", success_message);
            } else {
                match exit_status.code() {
                    Some(code) => println!("Exited with status code: {}", code),
                    None       => println!("Process terminated by signal")
                }
            }
        }
        _ => {
            eprint!("An error has occurred and the command returned no exit code!");
            exit(1)
        }
    }
}

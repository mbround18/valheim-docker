use std::process::{Command, exit};
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

        // let executable_path =.unwrap();
        // if executable_path.exists() {
        //     Option::from(Command::new(executable_path))
        // } else {
        //     eprint!("Failed to find {}", executable);
        //     None
        // }
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

pub fn execute_mut(command: &mut Command) {
    let status =  command
        .spawn()
        .unwrap()
        .wait();
    println!("Exited with status {:?}", status);
}

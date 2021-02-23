use log::{error, info};
use std::path::Path;
use std::process::{exit, Command, ExitStatus};

pub fn find_command(executable: &str) -> Option<Command> {
  let script_file = Path::new(executable);
  if script_file.exists() {
    info!("Executing: {} .....", executable.to_string());
    Option::from(Command::new(executable.to_string()))
  } else {
    match which::which(executable) {
      Ok(executable_path) => Option::from(Command::new(executable_path)),
      Err(_e) => {
        error!("Failed to find {} in path", executable);
        None
      }
    }
  }
}

pub fn create_execution(executable: &str) -> Command {
  match find_command(executable) {
    Some(command) => command,
    None => {
      error!("Unable to launch command {}", executable);
      exit(1)
    }
  }
}

pub fn execute_mut(command: &mut Command) -> std::io::Result<ExitStatus> {
  match command.spawn() {
    Ok(mut subprocess) => subprocess.wait(),
    _ => {
      error!("Failed to run process!");
      exit(1)
    }
  }
}

pub fn handle_exit_status(result: std::io::Result<ExitStatus>, success_message: String) {
  match result {
    Ok(exit_status) => {
      if exit_status.success() {
        info!("{}", success_message);
      } else {
        match exit_status.code() {
          Some(code) => info!("Exited with status code: {}", code),
          None => info!("Process terminated by signal"),
        }
      }
    }
    _ => {
      error!("An error has occurred and the command returned no exit code!");
      exit(1)
    }
  }
}

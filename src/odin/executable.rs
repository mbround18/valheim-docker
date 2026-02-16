use log::{debug, error, info};
use std::path::Path;
use std::process::{exit, Child, Command, ExitStatus};

pub fn find_command(executable: &str) -> Option<Command> {
  let script_file = Path::new(executable);
  if script_file.exists() {
    info!("Executing: {executable} .....");
    Option::from(Command::new(executable))
  } else {
    match which::which(executable) {
      Ok(executable_path) => Option::from(Command::new(executable_path)),
      Err(_e) => {
        error!("Failed to find {executable} in path");
        None
      }
    }
  }
}

pub fn create_execution(executable: &str) -> Command {
  match find_command(executable) {
    Some(command) => command,
    None => {
      error!("Unable to launch command {executable}");
      exit(1)
    }
  }
}

/// Parses a command line argument string according to shell-like rules
///
/// Returns a vector of individual argument strings after parsing according to these rules:
/// - Arguments without spaces are kept as-is
/// - Unquoted arguments with spaces are split into multiple arguments
/// - Quoted arguments (double or single quotes) are preserved as a single argument with quotes removed
/// - CLI style arguments like `--flag "value"` are parsed as separate arguments
pub fn parse_command_args(args: Vec<String>) -> Vec<String> {
  let mut parsed_args = Vec::new();

  for arg_str in args {
    let is_quoted = (arg_str.starts_with('"') && arg_str.ends_with('"'))
      || (arg_str.starts_with('\'') && arg_str.ends_with('\''));

    if arg_str.contains(' ') && !is_quoted {
      if let Some(pos) = arg_str.find(" \"") {
        let (flag, quoted_part) = arg_str.split_at(pos);
        parsed_args.push(flag.trim().to_string());
        parsed_args.push(
          quoted_part
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string(),
        );
      } else if let Some(pos) = arg_str.find(" '") {
        let (flag, quoted_part) = arg_str.split_at(pos);
        parsed_args.push(flag.trim().to_string());
        parsed_args.push(
          quoted_part
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string(),
        );
      } else {
        for part in arg_str.split_whitespace() {
          parsed_args.push(part.to_string());
        }
      }
    } else if is_quoted {
      parsed_args.push(arg_str.trim_matches('"').trim_matches('\'').to_string());
    } else {
      parsed_args.push(arg_str);
    }
  }

  parsed_args
}

pub fn execute_mut(command: &mut Command) -> std::io::Result<Child> {
  debug!("Running command: {:?}", command);

  command.spawn()
}

pub fn handle_exit_status(result: std::io::Result<ExitStatus>, success_message: String) {
  match result {
    Ok(exit_status) => {
      if exit_status.success() {
        info!("{success_message}");
      } else {
        match exit_status.code() {
          Some(code) => {
            error!("Exited with exit code: {code}");
            exit(code)
          }
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::process::Command;

  #[cfg(test)]
  fn get_processed_args(initial_args: &[&str]) -> Vec<String> {
    let mut cmd = Command::new("echo");

    for arg in initial_args {
      cmd.arg(arg);
    }

    let args: Vec<_> = cmd
      .get_args()
      .map(|arg| arg.to_string_lossy().into_owned())
      .collect();

    let parsed_args = parse_command_args(args);

    let mut processed_cmd = Command::new("echo");
    for arg in parsed_args {
      processed_cmd.arg(arg);
    }

    processed_cmd
      .get_args()
      .map(|arg| arg.to_string_lossy().into_owned())
      .collect()
  }

  #[test]
  fn test_simple_args() {
    let args = ["arg1", "arg2", "arg3"];
    let processed = get_processed_args(&args);
    assert_eq!(processed, vec!["arg1", "arg2", "arg3"]);
  }

  #[test]
  fn test_args_with_spaces() {
    let args = ["arg1 arg2", "arg3"];
    let processed = get_processed_args(&args);
    assert_eq!(processed, vec!["arg1", "arg2", "arg3"]);
  }

  #[test]
  fn test_quoted_args() {
    let args = ["\"arg1 arg2\"", "arg3"];
    let processed = get_processed_args(&args);
    // The quoted string should be preserved as one argument with quotes removed
    assert_eq!(processed, vec!["arg1 arg2", "arg3"]);
  }

  #[test]
  fn test_single_quoted_args() {
    let args = ["'arg1 arg2'", "arg3"];
    let processed = get_processed_args(&args);
    // The single-quoted string should be preserved as one argument with quotes removed
    assert_eq!(processed, vec!["arg1 arg2", "arg3"]);
  }

  #[test]
  fn test_mixed_args() {
    let args = ["simple", "with space", "\"quoted arg\"", "'single quoted'"];
    let processed = get_processed_args(&args);
    assert_eq!(
      processed,
      vec!["simple", "with", "space", "quoted arg", "single quoted"]
    );
  }

  #[test]
  fn test_cli_style_double_quoted_args() {
    let args = ["--flag \"value with spaces\""];
    let processed = get_processed_args(&args);
    assert_eq!(processed, vec!["--flag", "value with spaces"]);
  }

  #[test]
  fn test_cli_style_single_quoted_args() {
    let args = ["--flag 'value with spaces'"];
    let processed = get_processed_args(&args);
    assert_eq!(processed, vec!["--flag", "value with spaces"]);
  }

  #[test]
  fn test_multiple_cli_style_args() {
    let args = ["--flag1 \"value1\"", "--flag2 'value2'", "--flag3 value3"];
    let processed = get_processed_args(&args);
    assert_eq!(
      processed,
      vec!["--flag1", "value1", "--flag2", "value2", "--flag3", "value3"]
    );
  }

  #[test]
  fn test_find_command_exists() {
    // Test for a command that should definitely exist on Linux
    let cmd = find_command("ls");
    assert!(cmd.is_some());
  }

  #[test]
  fn test_find_command_not_exists() {
    let cmd = find_command("definitely_not_a_real_command_12345");
    assert!(cmd.is_none());
  }
}

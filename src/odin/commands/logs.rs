use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::common_paths::log_directory;
use log::{error, info, warn};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{read_to_string, File};
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::{error, fs};
use tokio::task;

/// Processes a line of text and generates appropriate log messages and notifications.
///
/// # Arguments
///
/// * `path` - A `PathBuf` representing the path of the file being processed.
/// * `line` - A `String` containing the line of text to be processed.
fn handle_line(path: PathBuf, line: String) {
  if line.is_empty() {
    return;
  }

  if line.contains("[Info   : Unity Log]") {
    // skipping duplicate lines
    return;
  }

  let file_name = Path::new(&path).file_name().unwrap().to_str().unwrap();

  if line.contains("WARNING") {
    warn!("[{}]: {}", file_name, line);
  } else if line.contains("ERROR") {
    error!("[{}]: {}", file_name, line);
  } else {
    info!("[{}]: {}", file_name, line);
  }

  if line.contains("Game server connected") {
    NotificationEvent::Start(crate::notifications::enums::event_status::EventStatus::Successful)
      .send_notification();
  }

  if line.contains("Steam manager on destroy") {
    NotificationEvent::Stop(crate::notifications::enums::event_status::EventStatus::Successful)
      .send_notification();
    info!("The game server has been stopped");
  }
}

fn read_file(file_name: String) -> Vec<u8> {
  let path = Path::new(&file_name);
  if !path.exists() {
    return String::from("Not Found!").into();
  }
  let mut file_content = Vec::new();
  let mut file = File::open(&file_name).expect("Unable to open file");
  file.read_to_end(&mut file_content).expect("Unable to read");
  file_content
}

async fn tail_file(path: PathBuf) -> Result<(), Box<dyn error::Error>> {
  let mut last_line = 0;
  let file = path.to_str().ok_or("Invalid file path")?;

  loop {
    let content = read_file(file.to_string());
    let current_line = content.lines().count();

    if current_line > last_line {
      content
        .lines()
        .skip(last_line)
        .for_each(|line| handle_line(path.clone(), line.unwrap_or_default()));

      last_line = current_line;
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
  }
}

pub async fn watch_logs(log_path: String) {
  let mut handles = Vec::new();
  let mut watched_files = HashMap::new();

  loop {
    let paths = fs::read_dir(&log_path)
      .expect("Could not read log directory")
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .collect::<Vec<_>>();

    for path in paths {
      if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("log") {
        watched_files.entry(path.clone()).or_insert_with(|| {
          let handle = task::spawn(async move {
            if let Err(e) = tail_file(path).await {
              error!("Error tailing file: {:?}", e);
            }
          });
          handles.push(handle);
        });
      }
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
  }
}

pub fn print_logs(log_path: String, lines: Option<u16>) {
  let paths = fs::read_dir(log_path)
    .expect("Could not read log directory")
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .collect::<Vec<_>>();

  for path in paths {
    if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("log") {
      let content = read_to_string(&path).expect("Could not read file");
      let lines = content
        .lines()
        .rev()
        .take(lines.unwrap_or(10) as usize)
        .collect::<Vec<_>>();
      for line in lines.iter().rev() {
        handle_line(path.clone(), line.to_string());
      }
    }
  }
}

pub async fn invoke(lines: Option<u16>, watch: bool) {
  let log_path = log_directory();
  let log_dir = PathBuf::from(&log_path);

  if !log_dir.exists() || !log_dir.is_dir() {
    error!("Log directory does not exist: {:?}", log_path);
    return;
  }

  if watch {
    watch_logs(log_path).await;
  } else {
    print_logs(log_path, lines);
  }
}

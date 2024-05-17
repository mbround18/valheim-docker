use crate::notifications::enums::notification_event::NotificationEvent;
use crate::utils::common_paths::log_directory;
use log::{error, info};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::{error, fs};
use tokio::task;

fn handle_line(path: PathBuf, line: &str) {
  if line.is_empty() {
    return;
  }

  let file_name = std::path::Path::new(&path)
    .file_name()
    .unwrap()
    .to_str()
    .unwrap();

  info!("[{}]: {}", file_name, line);

  if line.contains("Game server connected") {
    NotificationEvent::Start(crate::notifications::enums::event_status::EventStatus::Successful)
      .send_notification();
  }
}

async fn tail_file(path: PathBuf) -> Result<(), Box<dyn error::Error>> {
  let mut last_line = 0;
  let file = path.to_str().ok_or("Invalid file path")?;

  loop {
    let content = read_to_string(file)?;
    let current_line = content.lines().count();

    if current_line > last_line {
      content
        .lines()
        .skip(last_line)
        .for_each(|line| handle_line(path.clone(), line));

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
  let paths = fs::read_dir(&log_path)
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
        handle_line(path.clone(), line);
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

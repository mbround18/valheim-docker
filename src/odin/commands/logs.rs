use crate::log_filters::{handle_launch_probes, handle_player_events};
use crate::utils::common_paths::log_directory;
use crate::utils::environment::is_env_var_truthy;
use anyhow::{Context, Result};
use log::{error, info, warn};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task;

/// Struct to keep track of each file's state, including its last read line position.
#[derive(Clone)]
struct FileTracker {
  path: PathBuf,
  last_position: u64,
}

impl FileTracker {
  fn new(path: PathBuf) -> Self {
    Self {
      path,
      last_position: 0,
    }
  }
}

/// Processes a line of text from the log and generates appropriate log messages and notifications.
fn handle_line(path: &PathBuf, line: &str) {
  if line.trim().is_empty() {
    return;
  }

  if line.contains("[Info   : Unity Log]") {
    return;
  }

  if is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
    handle_player_events(line);
  }

  let file_name = match path.file_name().and_then(|name| name.to_str()) {
    Some(name) => name,
    None => {
      error!("Failed to extract file name from path: {:?}", path);
      return;
    }
  };

  if line.contains("WARNING") {
    warn!("[{}]: {}", file_name, line);
  } else if line.contains("ERROR") {
    error!("[{}]: {}", file_name, line);
  } else {
    info!("[{}]: {}", file_name, line);
  }

  handle_launch_probes(line);
}

/// Tails the given log file asynchronously, processing new lines as they are written to the file.
async fn tail_file(mut file_tracker: FileTracker) -> Result<()> {
  let file = File::open(&file_tracker.path).context("Unable to open file for tailing")?;
  let mut reader = BufReader::new(file);
  reader
    .seek(SeekFrom::Start(file_tracker.last_position))
    .context("Failed to seek to start position")?;

  loop {
    let mut new_lines = Vec::new();
    for line_result in reader.by_ref().lines() {
      match line_result {
        Ok(line) => {
          new_lines.push(line);
        }
        Err(e) => {
          error!(
            "Failed to read line from file '{}': {}",
            file_tracker.path.display(),
            e
          );
        }
      }
    }

    if !new_lines.is_empty() {
      file_tracker.last_position = reader.stream_position()?;
      for line in new_lines {
        handle_line(&file_tracker.path, &line);
      }
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
  }
}

pub async fn watch_logs(log_path: String) {
  let mut handles = Vec::new();
  let mut watched_files: HashMap<PathBuf, FileTracker> = HashMap::new();
  let log_path = Arc::new(log_path);

  loop {
    let paths = fs::read_dir(&*log_path)
      .expect("Could not read log directory")
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .collect::<Vec<_>>();

    for path in paths {
      if path.is_file() {
        watched_files.entry(path.clone()).or_insert_with(|| {
          let tracker = FileTracker::new(path.clone());
          let handle = task::spawn(tail_file(tracker.clone()));
          handles.push(handle);
          tracker
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
        handle_line(&path, line);
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

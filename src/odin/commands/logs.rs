use crate::log_filters::{handle_launch_probes, handle_player_events};
use crate::utils::common_paths::log_directory;
use crate::utils::environment::is_env_var_truthy;
use anyhow::{Context, Result};
use log::error;
use log::Level;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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

/// Returns true if the line appears to already be formatted by our Rust logger.
///
/// Detection rules:
/// - Known targets: `odin`, `huginn`, `shared`.
/// - Known levels: `info`, `debug`, `warn`, `warning`, `error`, `trace`.
/// - Matches module path style (e.g., `odin::module`), level+target with `:` or `::`
///   (e.g., `INFO odin: ...`, `INFO odin:: ...`), and loose `"<level> <target> ..."`.
pub(crate) fn is_already_formatted(line: &str) -> bool {
  let lower = line.to_ascii_lowercase();
  let lower_ws = lower.split_whitespace().collect::<Vec<_>>().join(" ");
  let levels = ["info", "debug", "warn", "warning", "error", "trace"];
  let targets = ["odin", "huginn", "shared"];
  if targets
    .iter()
    .any(|t| lower_ws.contains(&format!(" {t}::")))
  {
    return true;
  }
  for lvl in &levels {
    for tgt in &targets {
      let a = format!("{lvl} {tgt}:");
      let b = format!("{lvl} {tgt}::");
      let c = format!("{lvl} {tgt} ");
      if lower_ws.contains(&a) || lower_ws.contains(&b) || lower_ws.contains(&c) {
        return true;
      }
    }
  }
  false
}

/// Core formatter: processes a single logical line of text from the log and generates appropriate log messages and notifications.
fn handle_line_core(path: &PathBuf, line: &str) {
  if line.trim().is_empty() {
    return;
  }
  let outline = line.trim_end();
  if line.contains("[Info   : Unity Log]") {
    return;
  }

  if is_env_var_truthy("PLAYER_EVENT_NOTIFICATIONS") {
    handle_player_events(line);
  }

  let file_name = match path.file_name().and_then(|name| name.to_str()) {
    Some(name) => name,
    None => {
      error!("Failed to extract file name from path: {path:?}");
      return;
    }
  };

  if !is_env_var_truthy("SHOW_FALLBACK_HANDLER")
    && line.contains("Fallback handler could not load library")
  {
    return;
  }

  if !is_env_var_truthy("SHOW_SHADER_WARNINGS") && line.contains("WARNING: Shader") {
    return;
  }

  if is_already_formatted(outline) {
    handle_launch_probes(outline);
    return;
  }

  let level = if line.contains("WARNING") {
    Level::Warn
  } else if line.contains("ERROR") {
    Level::Error
  } else if line.contains("Fallback handler could not load library") {
    Level::Debug
  } else {
    Level::Info
  };

  log::log!(level, "[{file_name}]: {outline}");
  handle_launch_probes(outline);
}

/// Processes raw input that may contain carriage returns ("\r") used for in-place updates.
/// We split on "\r" and process each segment so progress-style logs are not squashed.
fn handle_line(path: &PathBuf, raw: &str) {
  if raw.contains('\r') {
    for segment in raw.split('\r') {
      if !segment.is_empty() {
        handle_line_core(path, segment);
      }
    }
  } else {
    handle_line_core(path, raw);
  }
}

/// Tails the given log file asynchronously, processing new lines as they are written.
async fn tail_file(mut file_tracker: FileTracker) -> Result<()> {
  let file = File::open(&file_tracker.path).context("Unable to open file for tailing")?;
  let mut reader = BufReader::new(file);
  reader
    .seek(SeekFrom::Start(file_tracker.last_position))
    .context("Failed to seek to start position")?;

  loop {
    let mut new_lines = Vec::new();
    loop {
      let mut buf = Vec::new();
      let bytes_read = reader
        .read_until(b'\n', &mut buf)
        .context("Failed to read from log file")?;
      if bytes_read == 0 {
        break;
      }
      let line = String::from_utf8_lossy(&buf).to_string();
      new_lines.push(line);
    }

    if !new_lines.is_empty() {
      file_tracker.last_position = reader
        .stream_position()
        .context("Failed to get stream position")?;
      for line in new_lines {
        handle_line(&file_tracker.path, &line);
      }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
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

    tokio::time::sleep(Duration::from_secs(2)).await;
  }
}

/// Prints the latest lines from `*.log` files in the provided directory.
///
/// - Reads files as raw bytes, converts to UTF-8 lossily.
/// - Defaults to last 10 lines per file if `lines` is `None`.
pub fn print_logs(log_path: String, lines: Option<u16>) {
  let paths = fs::read_dir(log_path)
    .expect("Could not read log directory")
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .collect::<Vec<_>>();

  for path in paths {
    if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("log") {
      let bytes = fs::read(&path).expect("Could not read file");
      let content = String::from_utf8_lossy(&bytes);
      let lines_to_print = content
        .lines()
        .rev()
        .take(lines.unwrap_or(10) as usize)
        .collect::<Vec<_>>();
      for line in lines_to_print.iter().rev() {
        handle_line(&path, line);
      }
    }
  }
}

/// Entrypoint used by the CLI: tails logs (`watch=true`) or prints recent lines.
///
/// Validates the log directory exists before proceeding.
pub async fn invoke(lines: Option<u16>, watch: bool) {
  let log_path = log_directory();
  let log_dir = PathBuf::from(&log_path);

  if !log_dir.exists() || !log_dir.is_dir() {
    error!("Log directory does not exist: {log_path:?}");
    return;
  }

  if watch {
    watch_logs(log_path).await;
  } else {
    print_logs(log_path, lines);
  }
}

#[cfg(test)]
mod tests {
  use super::is_already_formatted;

  #[test]
  fn detects_module_target_lines() {
    assert!(is_already_formatted(
      "2025-08-29T17:56:23.613579Z  INFO odin::files: Successfully written /home/steam/valheim/config.json"
    ));
    assert!(is_already_formatted(
      "2025-08-29T17:56:23.647870Z  INFO huginn: Starting web server...."
    ));
  }

  #[test]
  fn detects_plain_level_prefix_variants() {
    assert!(is_already_formatted("INFO  odin: something happened"));
    assert!(is_already_formatted("warning odin resource low"));
  }

  #[test]
  fn non_formatted_lines_are_false() {
    assert!(!is_already_formatted("[Valheim] Server started"));
    assert!(!is_already_formatted("Some random game output..."));
  }
}

use crate::log_filters::{handle_launch_probes, handle_player_events, handle_save_events};
use crate::utils::common_paths::log_directory;
use crate::utils::environment::is_env_var_truthy;
use anyhow::{Context, Result};
use log::Level;
use log::{error, warn};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

/// Follows one file the way `tail -F` does: it keeps reading as the file grows, and starts
/// over from the beginning when the file is truncated or replaced.
///
/// Valheim truncates `valheim_server.log` every time the server process starts, including
/// the in-process restarts done by `SCHEDULED_RESTART` and `odin update`. A forward-only
/// tail is left with an offset past the end of the new, shorter file and goes permanently
/// blind after the first restart, which silently disables every log filter driven from it.
struct LogTail {
  path: PathBuf,
  reader: Option<BufReader<File>>,
  /// Offset just past the last complete line handed out, into the currently open file.
  position: u64,
  /// Identifies the open file so a new file at the same path is noticed.
  file_id: Option<u64>,
}

impl LogTail {
  fn new(path: PathBuf) -> Self {
    Self {
      path,
      reader: None,
      position: 0,
      file_id: None,
    }
  }

  /// The inode of a file. `None` where the platform does not expose one, in which case
  /// replacement is not detected and only truncation is.
  fn identity(metadata: &fs::Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
      use std::os::unix::fs::MetadataExt;
      Some(metadata.ino())
    }
    #[cfg(not(unix))]
    {
      let _ = metadata;
      None
    }
  }

  /// True when the path no longer refers to the file being read, or that file has been
  /// truncated underneath the current offset.
  fn needs_reopen(&self, metadata: &fs::Metadata) -> bool {
    let replaced = matches!(
      (self.file_id, Self::identity(metadata)),
      (Some(open), Some(current)) if open != current
    );
    replaced || metadata.len() < self.position
  }

  fn open(&mut self, from: u64) -> Result<()> {
    let file = File::open(&self.path).context("Unable to open file for tailing")?;
    let metadata = file
      .metadata()
      .context("Unable to read metadata of file being tailed")?;
    let position = from.min(metadata.len());
    let mut reader = BufReader::new(file);
    reader
      .seek(SeekFrom::Start(position))
      .context("Failed to seek to start position")?;
    self.file_id = Self::identity(&metadata);
    self.position = position;
    self.reader = Some(reader);
    Ok(())
  }

  /// Reads every complete line available right now. A trailing line without its newline is
  /// still being written, so it is left in place rather than surfaced in two halves.
  fn read_available(&mut self) -> Result<Vec<String>> {
    let position = self.position;
    let Some(reader) = self.reader.as_mut() else {
      return Ok(Vec::new());
    };
    let mut lines = Vec::new();
    let mut offset = position;
    loop {
      let mut buf = Vec::new();
      let bytes_read = reader
        .read_until(b'\n', &mut buf)
        .context("Failed to read from log file")?;
      if bytes_read == 0 {
        break;
      }
      if !buf.ends_with(b"\n") {
        reader
          .seek(SeekFrom::Start(offset))
          .context("Failed to rewind to the start of an incomplete line")?;
        break;
      }
      offset += bytes_read as u64;
      lines.push(String::from_utf8_lossy(&buf).to_string());
    }
    self.position = offset;
    Ok(lines)
  }

  /// The lines written since the last call, reopening the file first if it was truncated or
  /// replaced. A file that has gone missing yields nothing and is picked up when it returns.
  fn poll(&mut self) -> Result<Vec<String>> {
    match fs::metadata(&self.path) {
      Ok(metadata) => {
        if self.reader.is_none() {
          self.open(0)?;
        } else if self.needs_reopen(&metadata) {
          warn!(
            "{} was truncated or replaced, following it from the start",
            self.path.display()
          );
          self.open(0)?;
        }
      }
      Err(_) => {
        self.reader = None;
        self.position = 0;
        self.file_id = None;
        return Ok(Vec::new());
      }
    }
    self.read_available()
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

  handle_player_events(line);
  handle_save_events(line);

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
  } else if line.contains("ERROR") || line.contains("Error saving world!") {
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
async fn tail_file(path: PathBuf) {
  let mut tail = LogTail::new(path.clone());
  loop {
    match tail.poll() {
      Ok(lines) => {
        for line in lines {
          handle_line(&path, &line);
        }
      }
      // Reading can fail while the server swaps the file out from under us; drop the handle
      // and try again on the next tick rather than ending the tail for the whole run.
      Err(e) => {
        error!("Error tailing {}: {e:?}", path.display());
        tail.reader = None;
      }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
  }
}

pub async fn watch_logs(log_path: String) {
  let mut handles = Vec::new();
  let mut watched_files: HashSet<PathBuf> = HashSet::new();
  let log_path = Arc::new(log_path);

  loop {
    let paths = fs::read_dir(&*log_path)
      .expect("Could not read log directory")
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .collect::<Vec<_>>();

    for path in paths {
      if path.is_file() && watched_files.insert(path.clone()) {
        handles.push(task::spawn(tail_file(path)));
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
  use super::{is_already_formatted, LogTail};
  use std::io::Write;

  fn append(path: &std::path::Path, contents: &str) {
    let mut file = std::fs::OpenOptions::new()
      .create(true)
      .append(true)
      .open(path)
      .expect("open for append");
    file.write_all(contents.as_bytes()).expect("append");
  }

  /// The regression behind #1533: Valheim truncates its log on every start, including the
  /// in-process restarts from `SCHEDULED_RESTART` and `odin update`.
  #[test]
  fn follows_the_file_across_truncation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("valheim_server.log");
    append(&path, "09/15/2026 18:00:00: first run line\n");

    let mut tail = LogTail::new(path.clone());
    assert_eq!(tail.poll().expect("first poll").len(), 1);
    assert!(tail.poll().expect("no new lines").is_empty());

    // The server restarts in place: same path, truncated to nothing, then written again.
    std::fs::write(&path, "").expect("truncate");
    append(&path, "[UnityMemory] Configuration Parameters\n");
    append(&path, "09/15/2026 18:16:18: Game server connected\n");

    let lines = tail.poll().expect("poll after truncation");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[1].contains("Game server connected"));
    assert!(tail.poll().expect("no new lines").is_empty());
  }

  #[test]
  fn follows_the_path_when_the_file_is_replaced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("valheim_server.log");
    append(&path, "old line one\nold line two\n");

    let mut tail = LogTail::new(path.clone());
    assert_eq!(tail.poll().expect("first poll").len(), 2);

    // Replaced by a longer file at the same path, so the length check alone would miss it.
    std::fs::remove_file(&path).expect("remove");
    append(&path, "new line one\nnew line two\nnew line three\n");

    let lines = tail.poll().expect("poll after replacement");
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].contains("new line one"));
  }

  #[test]
  fn a_missing_file_yields_nothing_and_is_picked_up_when_it_returns() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("valheim_server.log");

    let mut tail = LogTail::new(path.clone());
    assert!(tail
      .poll()
      .expect("missing file is not an error")
      .is_empty());

    append(&path, "here now\n");
    assert_eq!(tail.poll().expect("poll once created").len(), 1);
  }

  /// A line caught mid-write must not be surfaced as two lines, since the filters match on
  /// whole lines.
  #[test]
  fn an_incomplete_line_is_held_until_its_newline_arrives() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("valheim_server.log");
    append(&path, "complete\n09/15/2026 18:00:00: Got character ZDOID");

    let mut tail = LogTail::new(path.clone());
    let lines = tail.poll().expect("first poll");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("complete"));

    append(&path, " from Viking : 42:1\n");
    let lines = tail.poll().expect("second poll");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
      lines[0].trim_end(),
      "09/15/2026 18:00:00: Got character ZDOID from Viking : 42:1"
    );
  }

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

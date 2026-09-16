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
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom};
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
  /// The bytes immediately before `position`, used to notice that the content under the
  /// offset changed even when the file's length did not shrink below it.
  anchor: Vec<u8>,
}

/// How much of the last line handled is kept as the continuity anchor. Long enough that a
/// new run's output cannot plausibly match it, short enough to re-read on every poll.
const ANCHOR_BYTES: usize = 256;

impl LogTail {
  fn new(path: PathBuf) -> Self {
    Self {
      path,
      reader: None,
      position: 0,
      file_id: None,
      anchor: Vec::new(),
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
    self.anchor.clear();
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
    let mut anchor = None;
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
      anchor = Some(buf[buf.len().saturating_sub(ANCHOR_BYTES)..].to_vec());
      lines.push(String::from_utf8_lossy(&buf).to_string());
    }
    self.position = offset;
    if let Some(anchor) = anchor {
      self.anchor = anchor;
    }
    Ok(lines)
  }

  /// Confirms the bytes just before the current offset are still the ones that were read
  /// from there. Truncation is followed immediately by the new run writing its own output,
  /// so by the time the next poll comes around the file can already be longer than the old
  /// offset: the length check sees nothing wrong and the tail would resume from the middle
  /// of the new run, silently skipping everything before it.
  ///
  /// This compares content rather than hashing it, and it is a heuristic: output that
  /// repeats byte for byte with a period that happens to align with the anchor can still
  /// look continuous. Real server output carries timestamps, and the length check covers the
  /// ordinary case where the new run has not caught up yet, so the gap is theoretical.
  fn is_continuous(&mut self) -> Result<bool> {
    let position = self.position;
    let len = self.anchor.len();
    if len == 0 {
      return Ok(true);
    }
    let Some(reader) = self.reader.as_mut() else {
      return Ok(true);
    };
    reader
      .seek(SeekFrom::Start(position - len as u64))
      .context("Failed to seek back to the anchor")?;
    let mut buf = vec![0u8; len];
    let read = reader.read_exact(&mut buf);
    reader
      .seek(SeekFrom::Start(position))
      .context("Failed to seek back to the current position")?;
    match read {
      Ok(()) => Ok(buf == self.anchor),
      Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(false),
      Err(e) => Err(e).context("Failed to re-read the anchor"),
    }
  }

  /// The lines written since the last call, reopening the file first if it was truncated or
  /// replaced. A file that has gone missing yields nothing and is picked up when it returns.
  fn poll(&mut self) -> Result<Vec<String>> {
    match fs::metadata(&self.path) {
      Ok(metadata) => {
        if self.reader.is_none() {
          self.open(0)?;
        } else if self.needs_reopen(&metadata) || !self.is_continuous()? {
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
        self.anchor.clear();
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
  use super::{is_already_formatted, tail_file, LogTail, ANCHOR_BYTES};
  use serial_test::serial;
  use std::io::Write;
  use std::path::{Path, PathBuf};
  use std::time::{Duration, Instant};

  /// The tail end of a real `valheim_server.log` before a `SCHEDULED_RESTART`.
  const FIRST_RUN: &[&str] = &[
    "09/15/2026 08:41:12: Got character ZDOID from Skogsmaiden : 111222333:1",
    "09/15/2026 09:00:01: World save (527/527) done. Total time [143ms]",
    "09/15/2026 09:00:04: Destroying abandoned non persistent zdo 2130425389:1204 owner 2130425389",
    "09/15/2026 09:00:06: Shutting down",
    "09/15/2026 09:00:06: Steam manager on destroy",
  ];

  /// The head of the same file after the server restarted in place and truncated it. Note
  /// that the previous run's lines are gone, and the new run starts at offset 0.
  const SECOND_RUN: &[&str] = &[
    "[UnityMemory] Configuration Parameters - Can be set up in boot.config",
    "09/15/2026 09:00:31: Starting to load scene:start",
    "09/15/2026 09:01:02: Game server connected",
    "09/15/2026 09:12:44: Got character ZDOID from Viking : 2130425389:1",
  ];

  struct TempLog {
    _dir: tempfile::TempDir,
    path: PathBuf,
  }

  impl TempLog {
    fn new() -> Self {
      let dir = tempfile::tempdir().expect("tempdir");
      let path = dir.path().join("valheim_server.log");
      TempLog { _dir: dir, path }
    }

    fn append(&self, lines: &[&str]) {
      let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&self.path)
        .expect("open log for append");
      for line in lines {
        writeln!(file, "{line}").expect("append line");
      }
    }

    /// What Valheim does to this file on every server start: same path, same inode, length
    /// back to zero.
    fn truncate_in_place(&self) {
      std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&self.path)
        .expect("truncate log");
    }

    fn tail(&self) -> LogTail {
      LogTail::new(self.path.clone())
    }
  }

  /// Convenience: poll and strip the trailing newlines so expectations read as plain lines.
  fn poll(tail: &mut LogTail) -> Vec<String> {
    tail
      .poll()
      .expect("poll")
      .iter()
      .map(|line| line.trim_end().to_string())
      .collect()
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

  /// #1533: Valheim truncates its log on every server start, including the in-process
  /// restarts from `SCHEDULED_RESTART` and `odin update`. Every line of the new run has to
  /// be surfaced, exactly once and in order.
  #[test]
  fn surfaces_every_line_of_a_run_that_restarted_in_place() {
    let log = TempLog::new();
    log.append(FIRST_RUN);

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail), FIRST_RUN);

    log.truncate_in_place();
    log.append(SECOND_RUN);

    assert_eq!(poll(&mut tail), SECOND_RUN);
    assert!(poll(&mut tail).is_empty(), "the run must not be replayed");
  }

  /// The failure mode the length check alone does not catch: the restarted server writes
  /// past the old offset before the next poll, so the file is *longer* than it was even
  /// though it is a different run. Resuming from the old offset would skip the whole
  /// beginning of the new run, `Game server connected` included.
  #[test]
  fn detects_a_truncation_that_refilled_past_the_old_offset() {
    let log = TempLog::new();
    let long_first_run: Vec<String> = (0..40)
      .map(|i| {
        format!(
          "09/15/2026 09:00:0{}: World save ({i}/40) done. Total time [143ms]",
          i % 10
        )
      })
      .collect();
    log.append(
      &long_first_run
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>(),
    );

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail).len(), long_first_run.len());
    let old_position = std::fs::metadata(&log.path).expect("metadata").len();

    log.truncate_in_place();
    let second_run: Vec<String> = (0..80)
      .map(|i| format!("09/15/2026 09:01:0{}: second run line {i}", i % 10))
      .collect();
    log.append(&second_run.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(
      std::fs::metadata(&log.path).expect("metadata").len() > old_position,
      "this test is only meaningful while the new file is the longer one"
    );

    assert_eq!(poll(&mut tail), second_run);
  }

  /// Valheim emits the same world-save line over and over, so an anchor of one line would
  /// often match across runs by accident. It spans several lines, bounded by `ANCHOR_BYTES`
  /// so the re-read on every poll stays cheap.
  #[test]
  fn the_anchor_spans_more_than_one_short_line() {
    let log = TempLog::new();
    let repeated = "09/15/2026 09:00:01: World save (5/5) done. Total time [143ms]";
    log.append(&[repeated, repeated, repeated, repeated, repeated]);

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail).len(), 5);
    assert!(
      tail.anchor.len() > repeated.len(),
      "the anchor should span more than the last line when lines are short: {}",
      tail.anchor.len()
    );
    assert!(tail.anchor.len() <= ANCHOR_BYTES);

    log.truncate_in_place();
    log.append(&[repeated, "09/15/2026 09:01:02: Game server connected"]);
    let lines = poll(&mut tail);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[1].contains("Game server connected"));
  }

  /// A plain append, which is what happens for hours on end, must never reopen: a spurious
  /// reopen would replay the file and re-fire every player join and save-failure webhook in
  /// it.
  #[test]
  fn a_growing_file_is_never_replayed() {
    let log = TempLog::new();
    log.append(FIRST_RUN);

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail).len(), FIRST_RUN.len());

    for line in SECOND_RUN {
      log.append(&[line]);
      assert_eq!(poll(&mut tail), vec![line.to_string()]);
      assert!(poll(&mut tail).is_empty());
    }
  }

  /// Log rotation, or anything else that swaps a new file in at the same path. The new file
  /// is deliberately longer than the old one, so only the inode check catches it.
  #[test]
  fn follows_the_path_when_the_file_is_replaced() {
    let log = TempLog::new();
    log.append(&["old line one", "old line two"]);

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail).len(), 2);

    std::fs::remove_file(&log.path).expect("remove");
    log.append(&["new line one", "new line two", "new line three"]);

    let lines = poll(&mut tail);
    assert_eq!(
      lines,
      vec!["new line one", "new line two", "new line three"]
    );
  }

  /// The tail is started from the log directory before the server has written anything, so
  /// a missing file is normal and must not end the tail.
  #[test]
  fn a_missing_file_yields_nothing_and_is_picked_up_when_it_returns() {
    let log = TempLog::new();
    let mut tail = log.tail();

    assert!(poll(&mut tail).is_empty());
    assert!(poll(&mut tail).is_empty());

    log.append(&["here now"]);
    assert_eq!(poll(&mut tail), vec!["here now"]);

    std::fs::remove_file(&log.path).expect("remove");
    assert!(poll(&mut tail).is_empty());

    log.append(&["and again"]);
    assert_eq!(poll(&mut tail), vec!["and again"]);
  }

  /// A 100ms poll lands in the middle of a write eventually. The filters match whole lines,
  /// so half a line must be held back rather than surfaced and matched as two.
  #[test]
  fn an_incomplete_line_is_held_until_its_newline_arrives() {
    let log = TempLog::new();
    let mut file = std::fs::File::create(&log.path).expect("create");
    write!(file, "complete\n09/15/2026 09:12:44: Got character ZDOID").expect("partial write");
    file.flush().expect("flush");

    let mut tail = log.tail();
    assert_eq!(poll(&mut tail), vec!["complete"]);
    assert!(poll(&mut tail).is_empty(), "still incomplete");

    writeln!(file, " from Viking : 2130425389:1").expect("finish the line");
    file.flush().expect("flush");

    assert_eq!(
      poll(&mut tail),
      vec!["09/15/2026 09:12:44: Got character ZDOID from Viking : 2130425389:1"]
    );
  }

  /// Non UTF-8 bytes turn up in mod output; they must not abort the tail.
  #[test]
  fn invalid_utf8_is_read_lossily_rather_than_failing() {
    let log = TempLog::new();
    std::fs::write(&log.path, b"before\n\xff\xfe not utf8\nafter\n").expect("write");

    let mut tail = log.tail();
    let lines = poll(&mut tail);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "before");
    assert!(lines[1].contains("not utf8"));
    assert_eq!(lines[2], "after");
  }

  /// End to end over the real tail loop: the symptom reported in #1533 was that player
  /// events stopped arriving after a restart. `player.list` is what `handle_player_events`
  /// writes for every join, so its contents stand in for the webhook here.
  #[tokio::test]
  #[serial]
  async fn player_events_still_land_after_an_in_process_restart() {
    let saves = tempfile::tempdir().expect("tempdir");
    std::env::set_var(crate::constants::SAVE_LOCATION, saves.path());
    std::env::remove_var("WEBHOOK_URL");
    let player_list = saves.path().join("player.list");

    let log = TempLog::new();
    log.append(FIRST_RUN);
    let handle = tokio::spawn(tail_file(log.path.clone()));

    // The tail is live before the restart: the first run's join reaches the filters.
    let before = wait_for(&player_list, "Skogsmaiden").await;

    // The restart: Valheim truncates the log, the server comes back up, a player joins.
    log.truncate_in_place();
    log.append(SECOND_RUN);
    let after = wait_for(&player_list, "Viking").await;

    handle.abort();
    std::env::remove_var(crate::constants::SAVE_LOCATION);

    assert!(
      before,
      "the first run's join never reached the player filters"
    );
    assert!(
      after,
      "the join after the restart never reached the player filters; player.list: {:?}",
      std::fs::read_to_string(&player_list)
    );
  }

  async fn wait_for(path: &Path, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
      if std::fs::read_to_string(path).is_ok_and(|content| content.contains(needle)) {
        return true;
      }
      tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
  }
}

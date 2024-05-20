use std::env;

use log::{debug, Level, LevelFilter, Metadata, Record, SetLoggerError};

use crate::utils::parse_truthy::parse_truthy;

pub struct OdinLogger;

impl log::Log for OdinLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= Level::Debug
  }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      let prefix = format!(
        "{:width$}",
        format!("[ODIN][{}]", record.level()),
        width = 12
      );

      let args = record
        .args()
        .to_string()
        .replace('\n', &format!("\n{} - ", prefix));

      if args.contains("WARN") {
        println!("\x1b[33m{}: {}\x1b[0m", prefix, record.args());
      } else if args.contains("ERROR") {
        println!("\x1b[31m{}: {}\x1b[0m", prefix, record.args());
      } else {
        println!("{}: {}", prefix, record.args());
      }
    }
  }

  fn flush(&self) {}
}

static LOGGER: OdinLogger = OdinLogger;

pub fn initialize_logger(debug: bool) -> Result<(), SetLoggerError> {
  let level = if debug {
    LevelFilter::Debug
  } else {
    LevelFilter::Info
  };
  let result = log::set_logger(&LOGGER).map(|_| log::set_max_level(level));
  match result {
    Err(err) => {
      println!("Error setting logger: {:?}", err);
      Err(err)
    }
    Ok(_) => {
      debug!("Logger initialized");
      Ok(())
    }
  }
}

pub fn debug_mode() -> bool {
  let debug_mode = env::var("DEBUG_MODE").unwrap_or_default();
  parse_truthy(&debug_mode).unwrap_or(false)
}

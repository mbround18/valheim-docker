use log::{Level, Metadata, Record};

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
        width = 13
      );
      // This creates text blocks of logs if they include a new line.
      // I think it looks good <3
      let message = format!("{} - {}", prefix, record.args())
        .replace("\n", format!("\n{} - ", prefix).as_str());
      println!("{}", message);
    }
  }

  fn flush(&self) {}
}

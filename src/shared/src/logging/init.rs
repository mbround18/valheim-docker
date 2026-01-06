use std::env;
use tracing::subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::prelude::*; // brings SubscriberExt::with into scope
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_logging_and_tracing() -> Result<(), Box<dyn std::error::Error>> {
  // 1. Initialize LogTracer to bridge `log` messages to `tracing` events.
  //    We'll let the tracing Subscriber handle filtering based on RUST_LOG.
  // If another logger is already set, ignore the error to avoid panicking.
  let _ = LogTracer::init();

  // 2. Configure a tracing subscriber to format and output events.
  //    Default to INFO, and switch to DEBUG if DEBUG_MODE is truthy ("1", "true", "yes", "on").
  //    If RUST_LOG is set, it takes precedence.
  let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
    let debug_mode = env::var("DEBUG_MODE").unwrap_or_default();
    let is_debug = matches!(
      debug_mode.to_lowercase().as_str(),
      "1" | "true" | "yes" | "on"
    );
    let default_level = if is_debug { "debug" } else { "info" };
    // Append specific target overrides to silence noisy HTTP/2 and hyper internals by default.
    // Users can still override via RUST_LOG when needed.
    let default_filter = format!("{},h2=warn,hyper=warn", default_level);
    EnvFilter::new(default_filter)
  });

  let subscriber = tracing_subscriber::registry()
    .with(env_filter)
    .with(fmt::layer());

  // Set the global default subscriber without attempting to initialize the log tracer again.
  subscriber::set_global_default(subscriber)?;

  Ok(())
}

use crate::utils::environment::{fetch_var, is_env_var_truthy_with_default};
use std::time::Duration;

/// Set to `false`/`0` to serialize mod downloads: one mod at a time and no chunked
/// Range requests. Defaults to `true`. Useful when Thunderstore (or the CDN in front
/// of it) rate limits the burst of parallel requests.
pub const CONCURRENT_DOWNLOADS_ENABLED_VAR: &str = "CONCURRENT_DOWNLOADS_ENABLED";
/// Upper bound on requests in flight at once, across mods *and* range chunks.
pub const MAX_CONCURRENT_DOWNLOADS_VAR: &str = "MAX_CONCURRENT_DOWNLOADS";
/// Attempts made per request before a rate limited download is given up on.
pub const DOWNLOAD_RETRY_ATTEMPTS_VAR: &str = "DOWNLOAD_RETRY_ATTEMPTS";
/// Delay inserted between the start of each concurrent download, in milliseconds.
pub const DOWNLOAD_STAGGER_MS_VAR: &str = "DOWNLOAD_STAGGER_MS";
/// Idle keep-alive connections the pool retains per host.
pub const DOWNLOAD_POOL_MAX_IDLE_VAR: &str = "DOWNLOAD_POOL_MAX_IDLE_PER_HOST";
/// How long an idle pooled connection is kept, in seconds.
pub const DOWNLOAD_POOL_IDLE_TIMEOUT_VAR: &str = "DOWNLOAD_POOL_IDLE_TIMEOUT_SECS";
/// Whole-request timeout, in seconds.
pub const DOWNLOAD_REQUEST_TIMEOUT_VAR: &str = "DOWNLOAD_REQUEST_TIMEOUT_SECS";
/// TCP/TLS connect timeout, in seconds.
pub const DOWNLOAD_CONNECT_TIMEOUT_VAR: &str = "DOWNLOAD_CONNECT_TIMEOUT_SECS";

const DEFAULT_MAX_CONCURRENT_DOWNLOADS: usize = 4;
const DEFAULT_RETRY_ATTEMPTS: u32 = 5;
const DEFAULT_STAGGER_MS: u64 = 250;
const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 8;
const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 300;
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 15;

fn positive_var<T>(name: &str, default: T) -> T
where
  T: std::str::FromStr + PartialOrd + Default,
{
  fetch_var(name, "")
    .parse::<T>()
    .ok()
    .filter(|v| *v > T::default())
    .unwrap_or(default)
}

fn secs_var(name: &str, default: u64) -> Duration {
  Duration::from_secs(positive_var(name, default))
}

/// Whether mods may be downloaded in parallel. Defaults to `true`.
pub fn concurrent_downloads_enabled() -> bool {
  is_env_var_truthy_with_default(CONCURRENT_DOWNLOADS_ENABLED_VAR, true)
}

/// How many requests may be in flight at once, spanning both mod downloads and the
/// range chunks a single download splits into. Always `1` when
/// [`concurrent_downloads_enabled`] is false.
pub fn max_concurrent_downloads() -> usize {
  if !concurrent_downloads_enabled() {
    return 1;
  }
  positive_var(
    MAX_CONCURRENT_DOWNLOADS_VAR,
    DEFAULT_MAX_CONCURRENT_DOWNLOADS,
  )
}

pub fn retry_attempts() -> u32 {
  positive_var(DOWNLOAD_RETRY_ATTEMPTS_VAR, DEFAULT_RETRY_ATTEMPTS)
}

pub fn pool_max_idle_per_host() -> usize {
  positive_var(DOWNLOAD_POOL_MAX_IDLE_VAR, DEFAULT_POOL_MAX_IDLE_PER_HOST)
}

pub fn pool_idle_timeout() -> Duration {
  secs_var(
    DOWNLOAD_POOL_IDLE_TIMEOUT_VAR,
    DEFAULT_POOL_IDLE_TIMEOUT_SECS,
  )
}

pub fn request_timeout() -> Duration {
  secs_var(DOWNLOAD_REQUEST_TIMEOUT_VAR, DEFAULT_REQUEST_TIMEOUT_SECS)
}

pub fn connect_timeout() -> Duration {
  secs_var(DOWNLOAD_CONNECT_TIMEOUT_VAR, DEFAULT_CONNECT_TIMEOUT_SECS)
}

/// Spacing between concurrent request starts so we don't hit the origin with a burst.
/// Zero when downloads are already serialized.
pub fn download_stagger() -> Duration {
  if !concurrent_downloads_enabled() {
    return Duration::ZERO;
  }
  Duration::from_millis(
    fetch_var(DOWNLOAD_STAGGER_MS_VAR, "")
      .parse::<u64>()
      .unwrap_or(DEFAULT_STAGGER_MS),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  fn clear() {
    remove_var(CONCURRENT_DOWNLOADS_ENABLED_VAR);
    remove_var(MAX_CONCURRENT_DOWNLOADS_VAR);
    remove_var(DOWNLOAD_RETRY_ATTEMPTS_VAR);
    remove_var(DOWNLOAD_STAGGER_MS_VAR);
    remove_var(DOWNLOAD_POOL_MAX_IDLE_VAR);
    remove_var(DOWNLOAD_POOL_IDLE_TIMEOUT_VAR);
    remove_var(DOWNLOAD_REQUEST_TIMEOUT_VAR);
    remove_var(DOWNLOAD_CONNECT_TIMEOUT_VAR);
  }

  #[test]
  #[serial]
  fn pool_tunables_default_and_override() {
    clear();
    assert_eq!(pool_max_idle_per_host(), DEFAULT_POOL_MAX_IDLE_PER_HOST);
    assert_eq!(
      pool_idle_timeout(),
      Duration::from_secs(DEFAULT_POOL_IDLE_TIMEOUT_SECS)
    );
    assert_eq!(
      request_timeout(),
      Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS)
    );
    assert_eq!(
      connect_timeout(),
      Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS)
    );

    set_var(DOWNLOAD_POOL_MAX_IDLE_VAR, "2");
    set_var(DOWNLOAD_CONNECT_TIMEOUT_VAR, "7");
    assert_eq!(pool_max_idle_per_host(), 2);
    assert_eq!(connect_timeout(), Duration::from_secs(7));
    clear();
  }

  #[test]
  #[serial]
  fn concurrency_defaults_to_enabled() {
    clear();
    assert!(concurrent_downloads_enabled());
    assert_eq!(max_concurrent_downloads(), DEFAULT_MAX_CONCURRENT_DOWNLOADS);
    clear();
  }

  #[test]
  #[serial]
  fn disabling_serializes_downloads() {
    clear();
    for value in ["false", "0", "no", "off"] {
      set_var(CONCURRENT_DOWNLOADS_ENABLED_VAR, value);
      assert!(!concurrent_downloads_enabled(), "{value} should disable");
      assert_eq!(max_concurrent_downloads(), 1, "{value} should serialize");
      assert_eq!(download_stagger(), Duration::ZERO);
    }
    clear();
  }

  #[test]
  #[serial]
  fn explicit_limit_is_honoured_when_enabled() {
    clear();
    set_var(MAX_CONCURRENT_DOWNLOADS_VAR, "2");
    assert_eq!(max_concurrent_downloads(), 2);
    // A disabled flag always wins over an explicit limit.
    set_var(CONCURRENT_DOWNLOADS_ENABLED_VAR, "false");
    assert_eq!(max_concurrent_downloads(), 1);
    clear();
  }

  #[test]
  #[serial]
  fn invalid_values_fall_back_to_defaults() {
    clear();
    set_var(MAX_CONCURRENT_DOWNLOADS_VAR, "0");
    assert_eq!(max_concurrent_downloads(), DEFAULT_MAX_CONCURRENT_DOWNLOADS);
    set_var(MAX_CONCURRENT_DOWNLOADS_VAR, "banana");
    assert_eq!(max_concurrent_downloads(), DEFAULT_MAX_CONCURRENT_DOWNLOADS);
    set_var(DOWNLOAD_RETRY_ATTEMPTS_VAR, "-1");
    assert_eq!(retry_attempts(), DEFAULT_RETRY_ATTEMPTS);
    clear();
  }

  #[test]
  #[serial]
  fn stagger_defaults_and_overrides() {
    clear();
    assert_eq!(
      download_stagger(),
      Duration::from_millis(DEFAULT_STAGGER_MS)
    );
    set_var(DOWNLOAD_STAGGER_MS_VAR, "0");
    assert_eq!(download_stagger(), Duration::ZERO);
    clear();
  }
}

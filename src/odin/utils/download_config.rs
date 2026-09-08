use crate::utils::environment::{fetch_var, is_env_var_truthy_with_default};
use log::{debug, warn};
use reqwest::{RequestBuilder, Response, StatusCode};
use std::time::Duration;

/// Set to `false`/`0` to serialize mod downloads: one mod at a time and no chunked
/// Range requests. Defaults to `true`. Useful when Thunderstore (or the CDN in front
/// of it) rate limits the burst of parallel requests.
pub const CONCURRENT_DOWNLOADS_ENABLED_VAR: &str = "CONCURRENT_DOWNLOADS_ENABLED";
/// Upper bound on simultaneous downloads while concurrency is enabled.
pub const MAX_CONCURRENT_DOWNLOADS_VAR: &str = "MAX_CONCURRENT_DOWNLOADS";
/// Attempts made per request before a rate limited download is given up on.
pub const DOWNLOAD_RETRY_ATTEMPTS_VAR: &str = "DOWNLOAD_RETRY_ATTEMPTS";
/// Delay inserted between the start of each concurrent download, in milliseconds.
pub const DOWNLOAD_STAGGER_MS_VAR: &str = "DOWNLOAD_STAGGER_MS";

const DEFAULT_MAX_CONCURRENT_DOWNLOADS: usize = 4;
const DEFAULT_RETRY_ATTEMPTS: u32 = 5;
const DEFAULT_STAGGER_MS: u64 = 250;
const MAX_RETRY_DELAY_SECS: u64 = 60;

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

/// Whether mods may be downloaded in parallel. Defaults to `true`.
pub fn concurrent_downloads_enabled() -> bool {
  is_env_var_truthy_with_default(CONCURRENT_DOWNLOADS_ENABLED_VAR, true)
}

/// How many downloads (or Range chunks) may be in flight at once. Always `1` when
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

fn retry_attempts() -> u32 {
  positive_var(DOWNLOAD_RETRY_ATTEMPTS_VAR, DEFAULT_RETRY_ATTEMPTS)
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

/// Parses a `Retry-After` header, which is either a delay in seconds or an HTTP-date.
fn parse_retry_after(response: &Response) -> Option<Duration> {
  let raw = response.headers().get(reqwest::header::RETRY_AFTER)?;
  let raw = raw.to_str().ok()?.trim();

  if let Ok(secs) = raw.parse::<u64>() {
    return Some(Duration::from_secs(secs.min(MAX_RETRY_DELAY_SECS)));
  }

  let when = chrono::DateTime::parse_from_rfc2822(raw).ok()?;
  let delta = when.timestamp() - chrono::Utc::now().timestamp();
  Some(Duration::from_secs(
    (delta.max(0) as u64).min(MAX_RETRY_DELAY_SECS),
  ))
}

fn is_retryable(status: StatusCode) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Sends the request produced by `build`, honouring `Retry-After` when the origin rate
/// limits us and falling back to exponential backoff otherwise. `label` is only used
/// for logging. Non-retryable responses (including 4xx other than 429) are returned
/// as-is so callers keep their existing status handling.
pub async fn send_with_backoff<F>(label: &str, build: F) -> Result<Response, String>
where
  F: Fn() -> RequestBuilder,
{
  let attempts = retry_attempts();
  let mut backoff = Duration::from_secs(1);
  let mut last_err = String::new();

  for attempt in 1..=attempts {
    let delay = match build().send().await {
      Ok(response) => {
        if !is_retryable(response.status()) {
          return Ok(response);
        }
        let status = response.status();
        last_err = format!("{label}: status {status}");
        let delay = parse_retry_after(&response).unwrap_or(backoff);
        if attempt == attempts {
          warn!("{label}: {status} after {attempt} attempt(s); giving up");
          return Ok(response);
        }
        warn!(
          "{label}: {status} (attempt {attempt}/{attempts}); waiting {:.1}s before retry",
          delay.as_secs_f64()
        );
        delay
      }
      Err(e) => {
        last_err = format!("{label}: {e}");
        if attempt == attempts {
          break;
        }
        warn!(
          "{label}: request error (attempt {attempt}/{attempts}): {e}; waiting {:.1}s before retry",
          backoff.as_secs_f64()
        );
        backoff
      }
    };

    tokio::time::sleep(delay).await;
    backoff = (backoff * 2).min(Duration::from_secs(MAX_RETRY_DELAY_SECS));
  }

  debug!("{label}: exhausted {attempts} attempt(s)");
  Err(last_err)
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

  #[tokio::test]
  #[serial]
  async fn retries_429_then_succeeds() {
    clear();
    let mut server = mockito::Server::new_async().await;
    let rate_limited = server
      .mock("GET", "/pkg.zip")
      .with_status(429)
      .with_header("retry-after", "0")
      .expect(1)
      .create_async()
      .await;
    let ok = server
      .mock("GET", "/pkg.zip")
      .with_status(200)
      .with_body("ok")
      .create_async()
      .await;

    let url = format!("{}/pkg.zip", server.url());
    let client = reqwest::Client::new();
    let response = send_with_backoff("test", || client.get(&url))
      .await
      .unwrap();

    assert!(response.status().is_success());
    rate_limited.assert_async().await;
    ok.assert_async().await;
    clear();
  }

  #[tokio::test]
  #[serial]
  async fn returns_last_response_when_attempts_exhausted() {
    clear();
    set_var(DOWNLOAD_RETRY_ATTEMPTS_VAR, "2");
    let mut server = mockito::Server::new_async().await;
    let rate_limited = server
      .mock("GET", "/pkg.zip")
      .with_status(429)
      .with_header("retry-after", "0")
      .expect(2)
      .create_async()
      .await;

    let url = format!("{}/pkg.zip", server.url());
    let client = reqwest::Client::new();
    let response = send_with_backoff("test", || client.get(&url))
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    rate_limited.assert_async().await;
    clear();
  }
}

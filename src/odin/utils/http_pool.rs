use crate::utils::download_config::{
  connect_timeout, max_concurrent_downloads, pool_idle_timeout, pool_max_idle_per_host,
  request_timeout, retry_attempts,
};
use log::{debug, info, warn};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_RETRY_DELAY_SECS: u64 = 60;
const USER_AGENT: &str = "odin-valheim-docker/1.0 (+https://github.com/mbround18/valheim-docker)";

/// Tunables for the shared download pool, resolved from the environment once.
#[derive(Debug, Clone, Copy)]
pub struct PoolConfig {
  /// Requests allowed in flight at once, across every layer of the download work.
  pub max_concurrent: usize,
  /// Idle keep-alive connections retained per host for reuse.
  pub max_idle_per_host: usize,
  /// How long an idle connection is kept before being dropped.
  pub idle_timeout: Duration,
  /// Whole-request timeout, generous enough for large mod archives.
  pub request_timeout: Duration,
  /// TCP/TLS connect timeout.
  pub connect_timeout: Duration,
  /// Attempts per request before giving up.
  pub retry_attempts: u32,
}

impl PoolConfig {
  pub fn from_env() -> Self {
    Self {
      max_concurrent: max_concurrent_downloads(),
      max_idle_per_host: pool_max_idle_per_host(),
      idle_timeout: pool_idle_timeout(),
      request_timeout: request_timeout(),
      connect_timeout: connect_timeout(),
      retry_attempts: retry_attempts(),
    }
  }
}

/// Counters describing what the pool has done, for end-of-run reporting.
#[derive(Debug, Default)]
struct PoolStats {
  requests: AtomicU64,
  retries: AtomicU64,
  rate_limited: AtomicU64,
  failures: AtomicU64,
}

/// A point-in-time copy of the pool's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStatsSnapshot {
  pub requests: u64,
  pub retries: u64,
  pub rate_limited: u64,
  pub failures: u64,
}

/// Pooled HTTP client for Thunderstore traffic.
///
/// Three things are deliberately owned together here, because they only work as a set:
///
/// - **One connection pool.** Every request shares it, so repeat calls to Thunderstore
///   reuse established TLS connections instead of renegotiating per mod and per chunk.
/// - **One concurrency budget.** Bounding mod downloads and range chunks separately is
///   not enough - 4 mods each splitting into 4 chunks is 16 simultaneous requests, which
///   is what tripped Cloudflare to begin with. A single semaphore spans both layers, so
///   the real number of open requests never exceeds `max_concurrent` however the work
///   nests.
/// - **One rate-limit gate.** A 429 answers for the whole host, not just the unlucky
///   request that received it, so every in-flight worker pauses until the gate clears
///   rather than each rediscovering the limit on its own.
pub struct HttpPool {
  client: Client,
  permits: tokio::sync::Semaphore,
  /// Instant before which no new request may be sent, set from `Retry-After`.
  gate: Mutex<Option<Instant>>,
  stats: PoolStats,
  config: PoolConfig,
}

impl HttpPool {
  /// Builds a pool from an explicit config. Prefer [`HttpPool::global`]; this exists so
  /// tests can exercise a pool with its own budget and counters.
  pub fn with_config(config: PoolConfig) -> Self {
    let client = Client::builder()
      .user_agent(USER_AGENT)
      .timeout(config.request_timeout)
      .connect_timeout(config.connect_timeout)
      .pool_max_idle_per_host(config.max_idle_per_host)
      .pool_idle_timeout(config.idle_timeout)
      .build()
      .unwrap_or_else(|e| {
        warn!("Falling back to a default HTTP client: {e}");
        Client::new()
      });

    Self {
      client,
      permits: tokio::sync::Semaphore::new(config.max_concurrent),
      gate: Mutex::new(None),
      stats: PoolStats::default(),
      config,
    }
  }

  /// The process-wide pool. Configured on first use; a semaphore's permit count cannot
  /// be resized afterwards, so environment changes after the first request are ignored.
  pub fn global() -> &'static HttpPool {
    static POOL: OnceLock<HttpPool> = OnceLock::new();
    POOL.get_or_init(|| {
      let config = PoolConfig::from_env();
      debug!(
        "HTTP pool: {} concurrent request(s), {} idle conn(s)/host, {} attempt(s) per request",
        config.max_concurrent, config.max_idle_per_host, config.retry_attempts
      );
      HttpPool::with_config(config)
    })
  }

  pub fn config(&self) -> PoolConfig {
    self.config
  }

  /// Requests that may still start before the budget is exhausted.
  pub fn available_permits(&self) -> usize {
    self.permits.available_permits()
  }

  pub fn stats(&self) -> PoolStatsSnapshot {
    PoolStatsSnapshot {
      requests: self.stats.requests.load(Ordering::Relaxed),
      retries: self.stats.retries.load(Ordering::Relaxed),
      rate_limited: self.stats.rate_limited.load(Ordering::Relaxed),
      failures: self.stats.failures.load(Ordering::Relaxed),
    }
  }

  /// Logs a one-line summary, and calls out rate limiting with the flag that avoids it.
  pub fn log_summary(&self) {
    let stats = self.stats();
    if stats.requests == 0 {
      return;
    }
    info!(
      "HTTP pool: {} request(s), {} retry/retries, {} rate limited, {} failed",
      stats.requests, stats.retries, stats.rate_limited, stats.failures
    );
    if stats.rate_limited > 0 {
      warn!(
        "Thunderstore rate limited {} request(s). Set CONCURRENT_DOWNLOADS_ENABLED=false \
         or lower MAX_CONCURRENT_DOWNLOADS if this keeps happening.",
        stats.rate_limited
      );
    }
  }

  /// Blocks until any host-wide rate-limit pause has elapsed.
  async fn await_gate(&self) {
    loop {
      let wait = {
        let gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        gate.and_then(|until| until.checked_duration_since(Instant::now()))
      };
      match wait {
        Some(remaining) if !remaining.is_zero() => tokio::time::sleep(remaining).await,
        _ => return,
      }
    }
  }

  /// Records a host-wide pause, keeping the longest outstanding one.
  fn close_gate(&self, delay: Duration) {
    let until = Instant::now() + delay;
    let mut gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
    if gate.is_none_or(|current| until > current) {
      *gate = Some(until);
    }
  }

  /// Sends the request built by `build`, retrying rate limits and transport errors.
  ///
  /// `build` receives the pooled client so callers never construct their own. `label` is
  /// used only for logging. Responses that are not retryable - including 4xx other than
  /// 429 - are returned as-is so callers keep their own status handling.
  pub async fn execute<F>(&self, label: &str, build: F) -> Result<Response, String>
  where
    F: Fn(&Client) -> RequestBuilder,
  {
    let attempts = self.config.retry_attempts;
    let mut backoff = Duration::from_secs(1);
    let mut last_err = String::new();

    // Held only while the request is in flight; released once response headers arrive,
    // so bodies still stream in parallel while open requests stay bounded.
    let _permit = self
      .permits
      .acquire()
      .await
      .map_err(|e| format!("{label}: request permit closed: {e}"))?;

    for attempt in 1..=attempts {
      self.await_gate().await;
      self.stats.requests.fetch_add(1, Ordering::Relaxed);

      let delay = match build(&self.client).send().await {
        Ok(response) => {
          if !is_retryable(response.status()) {
            return Ok(response);
          }
          let status = response.status();
          last_err = format!("{label}: status {status}");

          // Retry-After is the server saying exactly how long to wait; honour it
          // verbatim and apply it host-wide. Our own backoff gets jitter instead.
          let delay = match parse_retry_after(&response) {
            Some(retry_after) => retry_after,
            None => with_jitter(backoff),
          };
          if status == StatusCode::TOO_MANY_REQUESTS {
            self.stats.rate_limited.fetch_add(1, Ordering::Relaxed);
            self.close_gate(delay);
          }
          if attempt == attempts {
            self.stats.failures.fetch_add(1, Ordering::Relaxed);
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
          let delay = with_jitter(backoff);
          warn!(
            "{label}: request error (attempt {attempt}/{attempts}): {e}; waiting {:.1}s before retry",
            delay.as_secs_f64()
          );
          delay
        }
      };

      self.stats.retries.fetch_add(1, Ordering::Relaxed);
      tokio::time::sleep(delay).await;
      backoff = (backoff * 2).min(Duration::from_secs(MAX_RETRY_DELAY_SECS));
    }

    self.stats.failures.fetch_add(1, Ordering::Relaxed);
    debug!("{label}: exhausted {attempts} attempt(s)");
    Err(last_err)
  }
}

fn is_retryable(status: StatusCode) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
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

/// Spreads retries so workers rate limited together do not all return at the same
/// instant. Dependency-free: the sub-second clock is random enough for jitter.
/// Returns `delay` scaled by roughly 0.5x-1.0x.
fn with_jitter(delay: Duration) -> Duration {
  if delay.is_zero() {
    return delay;
  }
  let entropy = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.subsec_nanos() as u64)
    .unwrap_or(0);
  let millis = delay.as_millis() as u64;
  Duration::from_millis(millis / 2 + entropy % (millis / 2 + 1))
}

/// Convenience wrapper over [`HttpPool::global`].
pub async fn send_with_backoff<F>(label: &str, build: F) -> Result<Response, String>
where
  F: Fn(&Client) -> RequestBuilder,
{
  HttpPool::global().execute(label, build).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;

  fn test_config(max_concurrent: usize, retry_attempts: u32) -> PoolConfig {
    PoolConfig {
      max_concurrent,
      max_idle_per_host: 4,
      idle_timeout: Duration::from_secs(30),
      request_timeout: Duration::from_secs(10),
      connect_timeout: Duration::from_secs(5),
      retry_attempts,
    }
  }

  #[test]
  fn jitter_stays_within_half_to_full() {
    for _ in 0..100 {
      let jittered = with_jitter(Duration::from_millis(800));
      assert!(
        jittered >= Duration::from_millis(400) && jittered <= Duration::from_millis(800),
        "jittered delay out of range: {jittered:?}"
      );
    }
    assert_eq!(with_jitter(Duration::ZERO), Duration::ZERO);
  }

  #[tokio::test]
  #[serial]
  async fn retries_429_then_succeeds() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
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

    let pool = HttpPool::with_config(test_config(4, 5));
    let url = format!("{}/pkg.zip", server.url());
    let response = pool
      .execute("test", |client| client.get(&url))
      .await
      .unwrap();

    assert!(response.status().is_success());
    limited.assert_async().await;
    ok.assert_async().await;

    let stats = pool.stats();
    assert_eq!(stats.requests, 2);
    assert_eq!(stats.retries, 1);
    assert_eq!(stats.rate_limited, 1);
    assert_eq!(stats.failures, 0);
  }

  #[tokio::test]
  #[serial]
  async fn returns_last_response_when_attempts_exhausted() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
      .mock("GET", "/pkg.zip")
      .with_status(429)
      .with_header("retry-after", "0")
      .expect(2)
      .create_async()
      .await;

    let pool = HttpPool::with_config(test_config(4, 2));
    let url = format!("{}/pkg.zip", server.url());
    let response = pool
      .execute("test", |client| client.get(&url))
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    limited.assert_async().await;
    assert_eq!(pool.stats().failures, 1);
  }

  #[tokio::test]
  #[serial]
  async fn non_retryable_status_returns_immediately() {
    let mut server = mockito::Server::new_async().await;
    let missing = server
      .mock("GET", "/pkg.zip")
      .with_status(404)
      .expect(1)
      .create_async()
      .await;

    let pool = HttpPool::with_config(test_config(4, 5));
    let url = format!("{}/pkg.zip", server.url());
    let response = pool
      .execute("test", |client| client.get(&url))
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    missing.assert_async().await;
    assert_eq!(pool.stats().retries, 0, "404 must not be retried");
  }

  /// The budget must span every caller, which is the whole point of a single pool:
  /// nested download + chunk work cannot exceed it.
  #[tokio::test]
  #[serial]
  async fn concurrency_budget_is_shared_across_callers() {
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    let mut server = mockito::Server::new_async().await;
    let _m = server
      .mock("GET", "/pkg.zip")
      .with_status(200)
      .with_body("ok")
      .expect_at_least(1)
      .create_async()
      .await;

    let pool = Arc::new(HttpPool::with_config(test_config(2, 1)));
    let url = Arc::new(format!("{}/pkg.zip", server.url()));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
      let pool = pool.clone();
      let url = url.clone();
      let in_flight = in_flight.clone();
      let peak = peak.clone();
      tasks.spawn(async move {
        pool
          .execute("test", |client| {
            let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(current, Ordering::SeqCst);
            let req = client.get(url.as_str());
            in_flight.fetch_sub(1, Ordering::SeqCst);
            req
          })
          .await
          .map(|r| r.status())
      });
    }
    while let Some(result) = tasks.join_next().await {
      assert!(result.unwrap().unwrap().is_success());
    }

    assert!(
      peak.load(Ordering::SeqCst) <= 2,
      "peak concurrency {} exceeded the budget of 2",
      peak.load(Ordering::SeqCst)
    );
    assert_eq!(pool.available_permits(), 2, "permits should be returned");
  }

  /// A 429 seen by one request pauses every other request against the host.
  #[tokio::test]
  #[serial]
  async fn rate_limit_gate_is_shared() {
    let pool = HttpPool::with_config(test_config(4, 1));
    pool.close_gate(Duration::from_millis(200));

    let started = Instant::now();
    pool.await_gate().await;
    let waited = started.elapsed();

    assert!(
      waited >= Duration::from_millis(150),
      "gate should have held the caller, waited only {waited:?}"
    );
  }

  #[test]
  fn gate_keeps_the_longest_pause() {
    let pool = HttpPool::with_config(test_config(4, 1));
    pool.close_gate(Duration::from_secs(30));
    let long = *pool.gate.lock().unwrap();
    pool.close_gate(Duration::from_secs(1));
    assert_eq!(
      *pool.gate.lock().unwrap(),
      long,
      "a shorter pause must not shorten an outstanding longer one"
    );
  }
}

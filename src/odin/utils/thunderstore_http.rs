use super::thunderstore_auth::with_thunderstore_auth;
use crate::errors::ValheimModError;
use reqwest::{RequestBuilder, Response, StatusCode};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Instant};

const MAX_RETRIES: u32 = 3;
static THUNDERSTORE: RequestGate =
  RequestGate::new(Duration::from_secs(1), Duration::from_secs(60));

struct RequestGate {
  next_request: Mutex<Option<Instant>>,
  spacing: Duration,
  retry_delay: Duration,
}

impl RequestGate {
  const fn new(spacing: Duration, retry_delay: Duration) -> Self {
    Self {
      next_request: Mutex::const_new(None),
      spacing,
      retry_delay,
    }
  }

  // Hold the gate through response headers so another worker cannot miss a new cooldown.
  async fn send(&self, request: RequestBuilder) -> Result<Response, ValheimModError> {
    for attempt in 0..=MAX_RETRIES {
      let mut next = self.next_request.lock().await;
      if let Some(deadline) = *next {
        sleep_until(deadline).await;
      }
      *next = Some(Instant::now() + self.spacing);
      let pending = request
        .try_clone()
        .ok_or_else(|| ValheimModError::DownloadError("Cannot retry mod request".into()))?
        .send();
      // Bound the wait for headers without limiting how long a download body can take.
      let response = tokio::time::timeout(Duration::from_secs(60), pending)
        .await
        .map_err(|_| {
          ValheimModError::DownloadError(
            "Timed out waiting for Thunderstore response headers".into(),
          )
        })?
        .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
      if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return Ok(response);
      }

      let delay = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| retry_after(value, SystemTime::now()))
        .unwrap_or(self.retry_delay * 2u32.pow(attempt))
        .max(self.spacing);
      let Some(deadline) = Instant::now().checked_add(delay) else {
        return Err(ValheimModError::DownloadError(
          "Thunderstore retry time is too far in the future".into(),
        ));
      };
      *next = Some(deadline);
      if attempt == MAX_RETRIES {
        return Ok(response);
      }
      log::warn!(
        "Thunderstore rate limit reached; waiting {} seconds before retrying ({}/{})",
        delay.as_secs(),
        attempt + 1,
        MAX_RETRIES
      );
      // Release the lock before retrying; all queued workers share the deadline.
    }
    unreachable!("the final attempt returns its response")
  }
}

fn retry_after(value: &str, now: SystemTime) -> Option<Duration> {
  let value = value.trim();
  if let Ok(seconds) = value.parse::<u64>() {
    return Some(Duration::from_secs(seconds));
  }
  let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
  let deadline: SystemTime = date.into();
  Some(deadline.duration_since(now).unwrap_or_default())
}

/// Pace Thunderstore lookups and downloads together, including retries after a 429.
/// Other hosts keep their existing request behavior.
pub(crate) async fn send(request: RequestBuilder, url: &str) -> Result<Response, ValheimModError> {
  let is_thunderstore = reqwest::Url::parse(url).ok().is_some_and(|url| {
    url
      .host_str()
      .is_some_and(|host| host == "thunderstore.io" || host.ends_with(".thunderstore.io"))
  });
  if is_thunderstore {
    THUNDERSTORE
      .send(with_thunderstore_auth(request, url))
      .await
  } else {
    request
      .send()
      .await
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reads_seconds_and_http_dates() {
    let now: SystemTime = chrono::DateTime::parse_from_rfc2822("Wed, 02 Sep 2015 07:28:00 GMT")
      .unwrap()
      .into();
    assert_eq!(retry_after("120", now), Some(Duration::from_secs(120)));
    assert_eq!(
      retry_after("Wed, 02 Sep 2015 07:30:00 GMT", now),
      Some(Duration::from_secs(120))
    );
    assert_eq!(
      retry_after("Wed, 02 Sep 2015 07:00:00 GMT", now),
      Some(Duration::ZERO)
    );
    assert_eq!(retry_after("invalid", now), None);
  }

  #[tokio::test]
  async fn retries_after_shared_cooldown() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
      .mock("GET", "/mod")
      .with_status(429)
      .with_header("Retry-After", "1")
      .expect(1)
      .create_async()
      .await;
    let success = server
      .mock("GET", "/mod")
      .with_status(200)
      .expect(2)
      .create_async()
      .await;
    let gate = RequestGate::new(Duration::from_millis(30), Duration::from_millis(10));
    let client = reqwest::Client::new();
    let start = Instant::now();
    let (first, second) = tokio::join!(
      gate.send(client.get(format!("{}/mod", server.url()))),
      gate.send(client.get(format!("{}/mod", server.url()))),
    );
    assert_eq!(first.unwrap().status(), StatusCode::OK);
    assert_eq!(second.unwrap().status(), StatusCode::OK);
    assert!(start.elapsed() >= Duration::from_millis(1030));
    limited.assert_async().await;
    success.assert_async().await;
  }

  #[tokio::test]
  async fn stops_after_bounded_retries_with_backoff() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
      .mock("GET", "/mod")
      .with_status(429)
      .with_header("Retry-After", "invalid")
      .expect(4)
      .create_async()
      .await;
    let gate = RequestGate::new(Duration::ZERO, Duration::from_millis(10));
    let start = Instant::now();
    let response = gate
      .send(reqwest::Client::new().get(format!("{}/mod", server.url())))
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(start.elapsed() >= Duration::from_millis(70));
    limited.assert_async().await;
  }

  #[tokio::test]
  async fn cdn_requests_retry_but_other_hosts_do_not() {
    let mut server = mockito::Server::new_async().await;
    let limited = server
      .mock("GET", "/mod")
      .with_status(429)
      .with_header("Retry-After", "0")
      .expect(1)
      .create_async()
      .await;
    let success = server
      .mock("GET", "/mod")
      .with_status(200)
      .expect(1)
      .create_async()
      .await;
    let client = reqwest::Client::new();
    let response = send(
      client.get(format!("{}/mod", server.url())),
      "https://cdn.thunderstore.io/mod.zip",
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    limited.assert_async().await;
    success.assert_async().await;

    let unrelated = server
      .mock("GET", "/other")
      .with_status(429)
      .expect(1)
      .create_async()
      .await;
    let response = send(
      client.get(format!("{}/other", server.url())),
      "https://thunderstore.io.example.com/mod.zip",
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    unrelated.assert_async().await;
  }
}

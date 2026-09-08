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

  // Keep other workers queued through retries so they cannot multiply the retry budget.
  async fn send(&self, request: RequestBuilder) -> Result<Response, ValheimModError> {
    let mut next = self.next_request.lock().await;
    for attempt in 0..=MAX_RETRIES {
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

/// Redirects are followed by `send` so each Thunderstore hop uses the shared gate.
pub(crate) fn client_builder() -> reqwest::ClientBuilder {
  reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
}

fn is_thunderstore(url: &reqwest::Url) -> bool {
  url
    .host_str()
    .is_some_and(|host| host == "thunderstore.io" || host.ends_with(".thunderstore.io"))
}

fn redirect_referer(
  previous: &reqwest::Url,
  next: &reqwest::Url,
) -> Option<reqwest::header::HeaderValue> {
  if previous.scheme() == "https" && next.scheme() == "http" {
    return None;
  }
  let mut referer = previous.clone();
  let _ = referer.set_username("");
  let _ = referer.set_password(None);
  referer.set_fragment(None);
  referer.as_str().parse().ok()
}

/// Send a mod GET request, pacing and retrying each Thunderstore redirect hop.
/// Callers must build their client with `client_builder`.
pub(crate) async fn send(request: RequestBuilder, url: &str) -> Result<Response, ValheimModError> {
  let (client, request) = with_thunderstore_auth(request, url).build_split();
  let mut request = request.map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
  for hop in 0..=10 {
    let builder = RequestBuilder::from_parts(
      client.clone(),
      request
        .try_clone()
        .ok_or_else(|| ValheimModError::DownloadError("Cannot retry mod request".into()))?,
    );
    let response = if is_thunderstore(request.url()) {
      THUNDERSTORE.send(builder).await?
    } else {
      builder
        .send()
        .await
        .map_err(|e| ValheimModError::DownloadError(e.to_string()))?
    };
    if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
      return Ok(response);
    }
    let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
      return Ok(response);
    };
    if hop == 10 {
      return Err(ValheimModError::DownloadError(
        "Too many mod download redirects".into(),
      ));
    }
    let location = location
      .to_str()
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
    let next = request
      .url()
      .join(location)
      .map_err(|e| ValheimModError::DownloadError(e.to_string()))?;
    if !matches!(next.scheme(), "http" | "https") {
      return Err(ValheimModError::DownloadError(
        "Unsupported mod redirect scheme".into(),
      ));
    }
    if request.url().origin() != next.origin() {
      // Match reqwest's protection against forwarding credentials to another origin.
      for header in [
        "authorization",
        "cookie",
        "cookie2",
        "proxy-authorization",
        "www-authenticate",
        "host",
      ] {
        request.headers_mut().remove(header);
      }
    }
    if let Some(referer) = redirect_referer(request.url(), &next) {
      request
        .headers_mut()
        .insert(reqwest::header::REFERER, referer);
    } else {
      request.headers_mut().remove(reqwest::header::REFERER);
    }
    *request.url_mut() = next;
  }
  unreachable!("the last redirect returns an error")
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
  async fn stops_queued_workers_after_bounded_retries_with_backoff() {
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
    let client = reqwest::Client::new();
    let url = format!("{}/mod", server.url());
    // Like the installer, cancel the other workers when the first request fails.
    let response = tokio::select! {
      result = gate.send(client.get(&url)) => result,
      result = gate.send(client.get(&url)) => result,
      result = gate.send(client.get(&url)) => result,
      result = gate.send(client.get(&url)) => result,
    }
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
    let client = client_builder()
      .no_proxy()
      .resolve("cdn.thunderstore.io", server.socket_address())
      .build()
      .unwrap();
    let url = format!(
      "http://cdn.thunderstore.io:{}/mod",
      server.socket_address().port()
    );
    let response = send(client.get(&url), &url).await.unwrap();
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

  #[tokio::test]
  async fn paces_redirects_and_drops_cross_origin_credentials() {
    let mut server = mockito::Server::new_async().await;
    let port = server.socket_address().port();
    let cdn_url = format!("http://cdn.thunderstore.io:{port}/redirect");
    let origin = server
      .mock("GET", "/origin")
      .with_status(302)
      .with_header("Location", &cdn_url)
      .expect(1)
      .create_async()
      .await;
    let redirect = server
      .mock("GET", "/redirect")
      .with_status(307)
      .match_header("authorization", mockito::Matcher::Missing)
      .match_header("cookie", mockito::Matcher::Missing)
      .match_header(
        "referer",
        format!("http://thunderstore.io:{port}/origin").as_str(),
      )
      .with_header("Location", "/final")
      .expect(1)
      .create_async()
      .await;
    let limited = server
      .mock("GET", "/final")
      .with_status(429)
      .with_header("Retry-After", "0")
      .expect(1)
      .create_async()
      .await;
    let success = server
      .mock("GET", "/final")
      .with_status(200)
      .match_header("authorization", mockito::Matcher::Missing)
      .match_header("Range", "bytes=0-3")
      .with_body("data")
      .expect(1)
      .create_async()
      .await;
    let client = client_builder()
      .no_proxy()
      .resolve("thunderstore.io", server.socket_address())
      .resolve("cdn.thunderstore.io", server.socket_address())
      .build()
      .unwrap();
    let url = format!("http://thunderstore.io:{port}/origin");
    let start = Instant::now();
    let response = send(
      client
        .get(&url)
        .basic_auth("test", Some("test"))
        .header("Cookie", "session=test")
        .header("Range", "bytes=0-3"),
      &url,
    )
    .await
    .unwrap();
    assert_eq!(response.url().path(), "/final");
    assert_eq!(response.text().await.unwrap(), "data");
    assert!(start.elapsed() >= Duration::from_secs(3));
    origin.assert_async().await;
    redirect.assert_async().await;
    limited.assert_async().await;
    success.assert_async().await;
  }

  #[tokio::test]
  async fn stops_redirect_loops() {
    let mut server = mockito::Server::new_async().await;
    let redirect = server
      .mock("GET", "/loop")
      .with_status(302)
      .with_header("Location", "/loop")
      .expect(11)
      .create_async()
      .await;
    let url = format!("{}/loop", server.url());
    let result = send(client_builder().build().unwrap().get(&url), &url).await;
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Too many mod download redirects"));
    redirect.assert_async().await;
  }

  #[test]
  fn sanitizes_referer_and_omits_it_on_https_downgrades() {
    let previous =
      reqwest::Url::parse("https://user:password@thunderstore.io/mod#private").unwrap();
    let next = reqwest::Url::parse("https://cdn.thunderstore.io/mod.zip").unwrap();
    assert_eq!(
      redirect_referer(&previous, &next).unwrap(),
      "https://thunderstore.io/mod"
    );
    let downgrade = reqwest::Url::parse("http://cdn.thunderstore.io/mod.zip").unwrap();
    assert!(redirect_referer(&previous, &downgrade).is_none());
  }
}

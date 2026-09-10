//! Redirect-aware sending for Thunderstore requests.
//!
//! Originally contributed by @skint007 in #1498. The pacing and retry logic that lived
//! here has moved into [`crate::utils::http_pool`], which applies one shared concurrency
//! budget and rate-limit gate across every request; what remains is the redirect walker,
//! which is the reason this module exists: following redirects manually means each hop is
//! paced, budgeted and retried on its own instead of disappearing inside a single
//! `send()`. Mod downloads redirect from `thunderstore.io` to the package CDN, so the hop
//! that actually transfers bytes is the one most worth pacing.
//!
//! Every mod repository in [`ModRepository`] goes through here, Hexium included, so each
//! one gets the same pacing, credentials scoping and redirect handling.

use super::mod_repository::{with_repository_auth, ModRepository};
use crate::errors::ValheimModError;
use crate::utils::http_pool::HttpPool;
use reqwest::{RequestBuilder, Response};

/// Redirect hops followed before giving up, matching reqwest's own default.
const MAX_REDIRECTS: usize = 10;

/// Headers that must never be forwarded across an origin change, mirroring reqwest's
/// protection against leaking credentials to another host.
const SENSITIVE_HEADERS: [&str; 6] = [
  "authorization",
  "cookie",
  "cookie2",
  "proxy-authorization",
  "www-authenticate",
  "host",
];

/// Builds the `Referer` for a redirect hop, or `None` when it must be dropped.
///
/// Credentials are stripped and the fragment removed, and the header is withheld entirely
/// on an https -> http downgrade so a secure URL is never disclosed over plaintext.
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

fn is_redirect(status: reqwest::StatusCode) -> bool {
  matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308)
}

/// Whether requests to `url` should go through the shared pool.
///
/// Deliberately wider than [`ModRepository::owns_host`], which gates *credentials*: a
/// `THUNDERSTORE_BASE_URL` or `HEXIUM_BASE_URL` override points at a mirror that needs the
/// same pacing and 429 handling, even though it must not receive our token.
fn should_pace(url: &reqwest::Url) -> bool {
  ModRepository::ALL.iter().any(|repo| repo.serves(url))
}

/// Sends a mod request, following redirects by hand so every hop is paced.
///
/// Hops to mod repository hosts go through the shared [`HttpPool`], picking up its
/// concurrency budget, rate-limit gate and `Retry-After` handling. Hops to anywhere else,
/// such as a GitHub release asset, are sent directly since those hosts are not the ones
/// rate limiting us.
pub(crate) async fn send(
  request: RequestBuilder,
  url: &str,
  label: &str,
) -> Result<Response, ValheimModError> {
  let (client, request) = with_repository_auth(request, url).build_split();
  let mut request = request.map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

  for hop in 0..=MAX_REDIRECTS {
    let response = if should_pace(request.url()) {
      HttpPool::global()
        .execute_request(label, &request)
        .await
        .map_err(ValheimModError::DownloadError)?
    } else {
      let attempt = request
        .try_clone()
        .ok_or_else(|| ValheimModError::DownloadError("Cannot retry mod request".into()))?;
      client
        .execute(attempt)
        .await
        .map_err(|e| ValheimModError::DownloadError(e.to_string()))?
    };

    if !is_redirect(response.status()) {
      return Ok(response);
    }
    let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
      return Ok(response);
    };
    if hop == MAX_REDIRECTS {
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
      for header in SENSITIVE_HEADERS {
        request.headers_mut().remove(header);
      }
    }
    match redirect_referer(request.url(), &next) {
      Some(referer) => {
        request
          .headers_mut()
          .insert(reqwest::header::REFERER, referer);
      }
      None => {
        request.headers_mut().remove(reqwest::header::REFERER);
      }
    }
    *request.url_mut() = next;
  }

  Err(ValheimModError::DownloadError(
    "Too many mod download redirects".into(),
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use reqwest::Url;
  use serial_test::serial;

  #[test]
  fn referer_strips_credentials_and_fragment() {
    let previous = Url::parse("https://user:secret@thunderstore.io/page/#frag").unwrap();
    let next = Url::parse("https://gcdn.thunderstore.io/file.zip").unwrap();
    let referer = redirect_referer(&previous, &next).expect("referer");
    let referer = referer.to_str().unwrap();
    assert!(!referer.contains("secret"), "credentials leaked: {referer}");
    assert!(!referer.contains("user"), "credentials leaked: {referer}");
    assert!(!referer.contains('#'), "fragment leaked: {referer}");
  }

  #[test]
  fn referer_dropped_on_downgrade_to_http() {
    let previous = Url::parse("https://thunderstore.io/page/").unwrap();
    let next = Url::parse("http://example.com/file.zip").unwrap();
    assert!(redirect_referer(&previous, &next).is_none());
  }

  #[tokio::test]
  #[serial]
  async fn follows_redirect_to_final_response() {
    let mut server = mockito::Server::new_async().await;
    let start = server
      .mock("GET", "/package/download/Author/Mod/1.0.0/")
      .with_status(302)
      .with_header("location", "/cdn/mod.zip")
      .expect(1)
      .create_async()
      .await;
    let final_hop = server
      .mock("GET", "/cdn/mod.zip")
      .with_status(200)
      .with_body("payload")
      .expect(1)
      .create_async()
      .await;

    let url = format!("{}/package/download/Author/Mod/1.0.0/", server.url());
    let client = reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .unwrap();
    let response = send(client.get(&url), &url, "test").await.unwrap();

    assert!(response.status().is_success());
    assert_eq!(response.text().await.unwrap(), "payload");
    start.assert_async().await;
    final_hop.assert_async().await;
  }

  /// A redirect that crosses origins must not carry the Authorization header with it.
  #[tokio::test]
  #[serial]
  async fn credentials_are_not_forwarded_across_origins() {
    let mut origin_a = mockito::Server::new_async().await;
    let mut origin_b = mockito::Server::new_async().await;

    let landing = origin_b
      .mock("GET", "/mod.zip")
      .match_header("authorization", mockito::Matcher::Missing)
      .with_status(200)
      .with_body("payload")
      .expect(1)
      .create_async()
      .await;
    let start = origin_a
      .mock("GET", "/start")
      .with_status(302)
      .with_header("location", &format!("{}/mod.zip", origin_b.url()))
      .expect(1)
      .create_async()
      .await;

    let url = format!("{}/start", origin_a.url());
    let client = reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .unwrap();
    let response = send(client.get(&url).bearer_auth("tss_secret"), &url, "test")
      .await
      .unwrap();

    assert!(response.status().is_success());
    start.assert_async().await;
    landing.assert_async().await;
  }
}

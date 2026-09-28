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

/// Whether following `previous` to `next` would drop TLS.
///
/// A base URL override that is plaintext to begin with stays allowed - only losing an
/// encrypted hop we already had counts, because that is the case where an attacker on the
/// path chooses the bytes we go on to unpack.
fn is_downgrade(previous: &reqwest::Url, next: &reqwest::Url) -> bool {
  previous.scheme() == "https" && next.scheme() == "http"
}

/// Whether a request must be paced regardless of the host it targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pacing {
  /// Pace mod repository hosts only. Anywhere else, such as a GitHub release asset, is
  /// sent directly because those hosts are not the ones rate limiting us.
  Auto,
  /// Always pace, for a third-party API that rate limits us but is not a mod repository.
  Always,
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
  send_with_pacing(request, url, label, Pacing::Auto).await
}

/// [`send`], with control over which hosts are paced.
///
/// [`Pacing::Always`] is for a host that rate limits us without being a mod repository, so
/// [`should_pace`] would otherwise send it direct and unretried.
pub(crate) async fn send_with_pacing(
  request: RequestBuilder,
  url: &str,
  label: &str,
  pacing: Pacing,
) -> Result<Response, ValheimModError> {
  let (client, request) = with_repository_auth(request, url).build_split();
  let mut request = request.map_err(|e| ValheimModError::DownloadError(e.to_string()))?;

  for hop in 0..=MAX_REDIRECTS {
    let response = if pacing == Pacing::Always || should_pace(request.url()) {
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
    // Refuse to lose TLS mid-download. Dropping the Referer is not enough: whatever the
    // plaintext hop returns is what we unpack, so a redirect off an https URL must stay
    // encrypted. Only the host is reported, to keep any credentials out of the message.
    if is_downgrade(request.url(), &next) {
      return Err(ValheimModError::DownloadError(format!(
        "Refusing mod redirect that drops TLS, to http://{}",
        next.host_str().unwrap_or("unknown host")
      )));
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
  fn downgrade_is_detected_only_when_tls_is_lost() {
    let cases = [
      ("https://thunderstore.io/a", "http://evil.test/b", true),
      ("https://thunderstore.io/a", "https://evil.test/b", false),
      // A mirror that was already plaintext keeps working; nothing was lost.
      ("http://localhost:4321/a", "http://localhost:4321/b", false),
      (
        "http://localhost:4321/a",
        "https://thunderstore.io/b",
        false,
      ),
    ];
    for (previous, next, expected) in cases {
      let previous = Url::parse(previous).unwrap();
      let next = Url::parse(next).unwrap();
      assert_eq!(
        is_downgrade(&previous, &next),
        expected,
        "{previous} -> {next}"
      );
    }
  }

  /// `Pacing::Always` must route a non-repository host through the pool so it still gets
  /// backoff. Without it the walker sends such hosts direct and unretried.
  #[tokio::test]
  #[serial]
  async fn pacing_always_retries_a_non_repo_host() {
    let mut server = mockito::Server::new_async().await;
    let flaky = server
      .mock("GET", "/profile/CODE")
      .with_status(503)
      .with_header("retry-after", "0")
      .expect(1)
      .create_async()
      .await;
    let ok = server
      .mock("GET", "/profile/CODE")
      .with_status(200)
      .with_body("zip")
      .expect(1)
      .create_async()
      .await;

    let url = format!("{}/profile/CODE", server.url());
    assert!(
      !should_pace(&Url::parse(&url).unwrap()),
      "the mock host must not be paced by host matching, or this proves nothing"
    );

    let client = reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .unwrap();
    let response = send_with_pacing(client.get(&url), &url, "gale", Pacing::Always)
      .await
      .unwrap();

    assert!(response.status().is_success());
    flaky.assert_async().await;
    ok.assert_async().await;
  }

  /// The same host under `Pacing::Auto` is sent direct, so a 503 comes straight back.
  #[tokio::test]
  #[serial]
  async fn pacing_auto_leaves_a_non_repo_host_unretried() {
    let mut server = mockito::Server::new_async().await;
    let once = server
      .mock("GET", "/profile/CODE")
      .with_status(503)
      .expect(1)
      .create_async()
      .await;

    let url = format!("{}/profile/CODE", server.url());
    let client = reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .unwrap();
    let response = send(client.get(&url), &url, "test").await.unwrap();

    assert_eq!(response.status().as_u16(), 503);
    once.assert_async().await;
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

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct ExternalResource {
  file: String,
  tag: String,
  url: String,
  attrs: HashMap<String, String>,
  is_stylesheet: bool,
}

fn static_file(path: &str) -> String {
  let full = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("static")
    .join(path);
  fs::read_to_string(&full).unwrap_or_else(|e| {
    panic!("failed to read static file {}: {}", full.display(), e);
  })
}

fn parse_external_resources(file: &str, html: &str) -> Vec<ExternalResource> {
  let tag_re = Regex::new(r#"<(?P<tag>script|link)\b(?P<attrs>[^>]*)>"#).unwrap();
  let attr_re =
    Regex::new(r#"(?P<name>[a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*"(?P<value>[^"]*)""#).unwrap();
  let rel_ws_re = Regex::new(r"\s+").unwrap();
  let mut resources = Vec::new();

  for cap in tag_re.captures_iter(html) {
    let tag = cap.name("tag").unwrap().as_str().to_lowercase();
    let attrs_src = cap.name("attrs").unwrap().as_str();
    let mut attrs = HashMap::<String, String>::new();

    for attr_cap in attr_re.captures_iter(attrs_src) {
      attrs.insert(
        attr_cap.name("name").unwrap().as_str().to_lowercase(),
        attr_cap.name("value").unwrap().as_str().to_string(),
      );
    }

    let url_key = if tag == "script" { "src" } else { "href" };
    let Some(url) = attrs.get(url_key).cloned() else {
      continue;
    };
    if !url.starts_with("https://") {
      continue;
    }

    let is_stylesheet = if tag == "link" {
      attrs
        .get("rel")
        .map(|v| {
          rel_ws_re
            .split(v)
            .any(|part| part.eq_ignore_ascii_case("stylesheet"))
        })
        .unwrap_or(false)
    } else {
      false
    };

    resources.push(ExternalResource {
      file: file.to_string(),
      tag,
      url,
      attrs,
      is_stylesheet,
    });
  }

  resources
}

fn host_from_url(url: &str) -> Option<&str> {
  let stripped = url.strip_prefix("https://")?;
  let end = stripped.find(['/', '?', '#']).unwrap_or(stripped.len());
  Some(&stripped[..end])
}

fn is_unpkg_pinned(url: &str) -> bool {
  Regex::new(r"@(\d+)\.(\d+)\.(\d+)").unwrap().is_match(url)
}

#[test]
fn external_resources_follow_security_policy() {
  let files = ["index.html", "swagger.html"];
  let mut failures = Vec::<String>::new();
  let no_sri_hosts = ["fonts.googleapis.com"];

  for file in files {
    let html = static_file(file);
    for resource in parse_external_resources(file, &html) {
      let Some(host) = host_from_url(&resource.url) else {
        failures.push(format!(
          "{}: malformed external URL {}",
          resource.file, resource.url
        ));
        continue;
      };

      if !resource.url.starts_with("https://") {
        failures.push(format!("{}: non-HTTPS URL {}", resource.file, resource.url));
      }

      if resource.tag == "script" || resource.is_stylesheet {
        let allow_no_sri = no_sri_hosts.iter().any(|h| h == &host);
        if !allow_no_sri {
          match resource.attrs.get("integrity") {
            Some(v) if v.starts_with("sha384-") => {}
            _ => failures.push(format!(
              "{}: {} missing sha384 integrity for {}",
              resource.file, resource.tag, resource.url
            )),
          }
          match resource.attrs.get("crossorigin") {
            Some(v) if v.eq_ignore_ascii_case("anonymous") => {}
            _ => failures.push(format!(
              "{}: {} missing crossorigin=\"anonymous\" for {}",
              resource.file, resource.tag, resource.url
            )),
          }
        }

        match resource.attrs.get("referrerpolicy") {
          Some(v) if v.eq_ignore_ascii_case("no-referrer") => {}
          _ => failures.push(format!(
            "{}: {} missing referrerpolicy=\"no-referrer\" for {}",
            resource.file, resource.tag, resource.url
          )),
        }
      }

      if host == "unpkg.com" && !is_unpkg_pinned(&resource.url) {
        failures.push(format!(
          "{}: unpkg URL must pin exact semver, found {}",
          resource.file, resource.url
        ));
      }
    }
  }

  assert!(
    failures.is_empty(),
    "static asset security policy violations:\n{}",
    failures.join("\n")
  );
}

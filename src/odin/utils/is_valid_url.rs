use reqwest::Url;

pub fn is_valid_url(input: &str) -> bool {
  Url::parse(input).is_ok()
}

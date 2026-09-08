use std::fmt::Error;

pub fn parse_truthy(value: &str) -> Result<bool, Error> {
  Ok(match value.to_lowercase().as_str() {
    "true" | "1" | "yes" | "y" | "on" => true,
    "false" | "0" | "no" | "n" | "off" => false,
    _ => false,
  })
}

// test the parse_truthy function
#[test]
fn test_parse_truthy() {
  assert_eq!(parse_truthy("true"), Ok(true));
  assert_eq!(parse_truthy("false"), Ok(false));
  assert_eq!(parse_truthy("1"), Ok(true));
  assert_eq!(parse_truthy("0"), Ok(false));
  assert_eq!(parse_truthy("yes"), Ok(true));
  assert_eq!(parse_truthy("ON"), Ok(true));
  assert_eq!(parse_truthy("no"), Ok(false));
  assert_eq!(parse_truthy("off"), Ok(false));
  assert_eq!(parse_truthy(""), Ok(false));
  assert_eq!(parse_truthy("qwdqwdqwd"), Ok(false));
}

use regex::Regex;

/// Parses a mod string into its author, mod name, and version components.
///
/// # Arguments
///
/// * `mod_string` - A string slice that holds the mod string to be parsed.
///
/// # Returns
///
/// An `Option` containing a tuple with the author, mod name, and version if parsing is successful; `None` otherwise.
pub fn parse_mod_string(mod_string: &str) -> Option<(&str, &str, &str)> {
  // Support versions with digits and dots, and wildcard placeholders '*' or 'x'
  // Examples:
  //  - denikson-BepInExPack_Valheim-5.4.2202
  //  - Author-Mod-*
  //  - Author-Mod-1.*
  //  - Author-Mod-1.2.*
  //  - Author-Mod-x
  //  - Author-Mod-1.x
  //  - Author-Mod-1.2.x
  let re = Regex::new(r"^([^-\s]+)-([^-]+)-([0-9xX\.*]+)$").unwrap();
  re.captures(mod_string).and_then(|caps| {
    if caps.len() == 4 {
      Some((
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
      ))
    } else {
      None
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_valid_mod_strings() {
    let mod_strings = [
      "denikson-BepInExPack_Valheim-5.4.2202",
      "RandyKnapp-EpicLoot-0.10.3",
      "ValheimModding-Jotunn-2.23.2",
      "Advize-PlantEverything-1.18.2",
      // Wildcard variants
      "Author-Mod-*",
      "Author-Mod-x",
      "Author-Mod-1.*",
      "Author-Mod-1.x",
      "Author-Mod-1.2.*",
      "Author-Mod-1.2.x",
    ];

    for mod_str in &mod_strings {
      let result = parse_mod_string(mod_str);
      assert!(
        result.is_some(),
        "Failed to parse valid mod string: {mod_str}"
      );
    }
  }

  #[test]
  fn test_invalid_mod_strings() {
    let mod_strings = [
      "InvalidModString",
      "AnotherInvalid-One",
      "Missing-Version-",
      "-MissingAuthor-1.0.0",
    ];

    for mod_str in &mod_strings {
      let result = parse_mod_string(mod_str);
      assert!(result.is_none(), "Parsed invalid mod string: {mod_str}");
    }
  }

  #[test]
  fn test_parse_components() {
    let mod_str = "denikson-BepInExPack_Valheim-5.4.2202";
    let result = parse_mod_string(mod_str).expect("Failed to parse mod string");
    assert_eq!(result.0, "denikson");
    assert_eq!(result.1, "BepInExPack_Valheim");
    assert_eq!(result.2, "5.4.2202");
  }
}

use crate::{
  files::{FileManager, ManagedFile},
  notifications::discord::{DiscordWebHookBody},
  utils::{environment::fetch_var, path_exists},
};

use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ODIN_DISCORD_FILE_VAR: &str = "ODIN_DISCORD_FILE";

#[derive(Deserialize, Serialize)]
pub struct DiscordConfigEvents {
  broadcast: DiscordWebHookBody,
  start: DiscordWebHookBody,
  stop: DiscordWebHookBody,
  update: DiscordWebHookBody,
  player_join: DiscordWebHookBody,
  player_leave: DiscordWebHookBody,
}

#[derive(Deserialize, Serialize)]
pub struct DiscordConfig {
  pub(crate) events: HashMap<String, DiscordWebHookBody>,
}

fn basic_template() -> DiscordConfig {
  let mut events: HashMap<String, DiscordWebHookBody> = HashMap::new();
  events.insert(String::from("broadcast"), DiscordWebHookBody::default());
  events.insert(String::from("start"), DiscordWebHookBody::default());
  events.insert(String::from("stop"), DiscordWebHookBody::default());
  events.insert(String::from("update"), DiscordWebHookBody::default());
  events.insert(String::from("player_join"), DiscordWebHookBody::default());
  events.insert(String::from("player_leave"), DiscordWebHookBody::default());
  DiscordConfig { events }
}

pub fn load_discord() -> DiscordConfig {
  let file = discord_file();
  read_discord(&file)
}

pub fn discord_file() -> ManagedFile {
  let name = fetch_var(ODIN_DISCORD_FILE_VAR, "discord.json");
  debug!("Config file set to: {}", name);
  ManagedFile { name }
}

pub fn read_discord(discord: &dyn FileManager) -> DiscordConfig {
  let default = basic_template();
  let content = discord.read();
  if content.is_empty() {
    return default;
  }

  let config =
    serde_json::from_str::<HashMap<String, HashMap<String, DiscordWebHookBody>>>(content.as_str())
      .unwrap()
      .get("events")
      .unwrap_or(&default.events)
      .clone();
  let mut events: HashMap<String, DiscordWebHookBody> = HashMap::new();

  for (key, value) in default.events {
    if !config.contains_key(&key) {
      {
        events.insert(key, value);
      }
    } else if let Some(return_value) = config.get(&key) {
      events.insert(key, return_value.clone());
    } else {
      events.insert(key, DiscordWebHookBody::default());
    }
  }

  DiscordConfig { events }
}

pub fn write_discord(discord: &dyn FileManager) -> bool {
  if path_exists(&discord.path()) {
    debug!("Discord config file already exists, doing nothing.");
    return true; 
  }

  let template_notification = basic_template();
  let content_to_write = serde_json::to_string_pretty(&template_notification).unwrap();
  debug!("Writing discord config: \n{}", content_to_write);

  discord.write(content_to_write)
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockall::*;

  mock! {
      ManagedFile {
          fn read(&self) -> String;
          fn write(&self, content: String) -> bool;
          fn path(&self) -> String;
      }
  }

  impl FileManager for MockManagedFile {
    fn path(&self) -> String {
      self.path()
    }

    fn read(&self) -> String {
      self.read()
    }

    fn write(&self, content: String) -> bool {
      self.write(content)
    }
  }

  #[test]
  fn test_basic_template() {
    let config = basic_template();
    assert!(config.events.contains_key("broadcast"));
    assert!(config.events.contains_key("start"));
    assert!(config.events.contains_key("stop"));
    assert!(config.events.contains_key("update"));
    assert!(config.events.contains_key("player_join"));
    assert!(config.events.contains_key("player_leave"));
  }

  #[test]
  fn test_read_discord_with_empty_content() {
    let mut mock_file = MockManagedFile::new();
    mock_file.expect_read().return_const(String::from(""));

    let config = read_discord(&mock_file);

    assert!(config.events.contains_key("broadcast"));
    assert!(config.events.contains_key("start"));
    assert!(config.events.contains_key("stop"));
    assert!(config.events.contains_key("update"));
    assert!(config.events.contains_key("player_join"));
    assert!(config.events.contains_key("player_leave"));
  }

  #[test]
  fn test_read_discord_with_valid_content() {
    let content = r#"
        {
            "events": {
                "broadcast": {
                    "content": "test_broadcast",
                    "embeds": [{
                        "title": "Test Title",
                        "description": "Test Description",
                        "color": 12345
                    }]
                }
            }
        }"#;

    let mut mock_file = MockManagedFile::new();
    mock_file.expect_read().return_const(String::from(content));

    let config = read_discord(&mock_file);

    assert_eq!(
      config.events.get("broadcast").unwrap().content,
      "test_broadcast"
    );
    assert_eq!(
      config.events.get("broadcast").unwrap().embeds[0].title,
      "Test Title"
    );
    assert_eq!(
      config.events.get("broadcast").unwrap().embeds[0].description,
      "Test Description"
    );
    assert_eq!(
      config.events.get("broadcast").unwrap().embeds[0].color,
      12345
    );
  }

  #[test]
  fn test_write_discord_when_path_does_not_exist() {
    let mut mock_file = MockManagedFile::new();
    mock_file
      .expect_write()
      .withf(|content| content.contains("broadcast"))
      .return_const(true);

    mock_file
      .expect_path()
      .return_const(String::from("non_existing_path"));

    let result = write_discord(&mock_file);
    assert!(
      result,
      "Expected write to return true when path does not exist."
    );
  }

  #[test]
  fn test_read_discord_without_events_key() {
    let mut mock_file = MockManagedFile::new();
    let no_events_content = r#"{}"#;
    mock_file.expect_read().return_const(String::from(no_events_content));

    let config = read_discord(&mock_file);

    assert!(config.events.contains_key("broadcast"));
    assert!(config.events.contains_key("start"));
  }


  #[test]
  fn test_read_discord_with_extra_event_keys() {
    let mut mock_file = MockManagedFile::new();
    let extra_keys_content = r#"
    {
        "events": {
            "extra_event": {
                "title": "test",
                "content": "extra_content",
                "embeds": [{
                    "title": "title",
                    "description": "Test Description",
                    "color": 12345
                }]
            }
        }
    }"#;
    mock_file.expect_read().return_const(String::from(extra_keys_content));

    let config = read_discord(&mock_file);

    assert!(!config.events.contains_key("extra_event"), "Unexpected key should be ignored");
    assert!(config.events.contains_key("broadcast"));
    assert!(config.events.contains_key("start"));
  }



  #[test]
  fn test_read_discord_with_malformed_json() {
    let mut mock_file = MockManagedFile::new();
    let malformed_json = r#"{ "events": { "broadcast": { "content": "test_broadcast""#;
    mock_file.expect_read().return_const(String::from(malformed_json));

    let result = std::panic::catch_unwind(|| read_discord(&mock_file));

    assert!(result.is_err(), "Expected a panic when the JSON is malformed");
  }


  #[test]
  fn test_discord_file() {
    let managed_file = discord_file();
    assert_eq!(managed_file.name, "discord.json");
  }
}

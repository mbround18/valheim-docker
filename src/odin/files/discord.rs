use crate::{
  files::{FileManager, ManagedFile},
  notifications::discord::{body_template, DiscordWebHookBody},
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
}

#[derive(Deserialize, Serialize)]
pub struct DiscordConfig {
  pub(crate) events: HashMap<String, DiscordWebHookBody>,
}

fn basic_template() -> DiscordConfig {
  let events: HashMap<String, DiscordWebHookBody> = HashMap::new();
  let mut config = DiscordConfig { events };
  config
    .events
    .insert(String::from("broadcast"), body_template());
  config.events.insert(String::from("start"), body_template());
  config.events.insert(String::from("stop"), body_template());
  config
    .events
    .insert(String::from("update"), body_template());
  config
}

pub fn load_discord() -> DiscordConfig {
  let file = discord_file();
  read_discord(file)
}

pub fn discord_file() -> ManagedFile {
  let name = fetch_var(ODIN_DISCORD_FILE_VAR, "discord.json");
  debug!("Config file set to: {}", name);
  ManagedFile { name }
}

pub fn read_discord(discord: ManagedFile) -> DiscordConfig {
  let content = discord.read();
  if content.is_empty() {
    basic_template()
  } else {
    serde_json::from_str(content.as_str()).unwrap()
  }
}

pub fn write_discord(discord: ManagedFile) -> bool {
  let notification = basic_template();
  let content_to_write = serde_json::to_string_pretty(&notification).unwrap();
  debug!(
    "Writing discord config: \n{}",
    serde_json::to_string_pretty(&notification).unwrap()
  );
  if path_exists(&discord.path()) {
    false
  } else {
    discord.write(content_to_write)
  }
}

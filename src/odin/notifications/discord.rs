use crate::files::{
  config::{config_file, read_config},
  FileManager,
};
use crate::notifications::EventStatus;
use crate::notifications::NotificationMessage;
use inflections::case::to_title_case;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{env, str::FromStr};

#[derive(Debug)]
enum Color {
  Success = 0x4B_B5_43,
  Failure = 0xFA_11_3D,
  Generic = 0x00_7F_66,
}

const DISCORD_WEBHOOK_BASE: &str = "https://discord.com/api/webhooks";
const DISCORDAPP_WEBHOOK_BASE: &str = "https://discordapp.com/api/webhooks";

impl From<EventStatus> for Color {
  fn from(event: EventStatus) -> Self {
    use EventStatus::{Failed, Successful};
    match event {
      Successful => Self::Success,
      Failed => Self::Failure,
      _ => Self::Generic,
    }
  }
}

pub fn is_discord_webhook(webhook_url: &str) -> bool {
  webhook_url.starts_with(DISCORD_WEBHOOK_BASE) || webhook_url.starts_with(DISCORDAPP_WEBHOOK_BASE)
}

#[derive(Deserialize, Serialize)]
pub struct DiscordWebHookEmbed {
  title: String,
  description: String,
  color: i32,
}

#[derive(Deserialize, Serialize)]
pub struct DiscordWebHookBody {
  content: String,
  embeds: Vec<DiscordWebHookEmbed>,
}

impl DiscordWebHookBody {
  pub fn new(event: &NotificationMessage) -> Self {
    // Some contexts currently don't get passed in $NAME so fall back to reading from the config
    // if it's missing or invalid UTF-8
    let server_name = match env::var("NAME") {
      Ok(name) if !name.is_empty() => name,
      _ => {
        let config_file = config_file();
        debug!(
          "Empty or missing $NAME. Falling back to reading from {}",
          config_file.path()
        );
        let config = read_config(config_file);
        config.name
      }
    };
    let status = &event.event_type.status;
    let event_status = EventStatus::from_str(status).unwrap_or(EventStatus::Failed);
    let color: i32 = Color::from(event_status) as i32;
    let payload = DiscordWebHookBody {
      content: to_title_case(format!("Notification From: {}", server_name).as_str()),
      embeds: vec![DiscordWebHookEmbed {
        title: String::from(&event.event_type.name),
        description: String::from(&event.event_message),
        color,
      }],
    };
    debug!(
      "Discord Payload: {}",
      serde_json::to_string(&payload).unwrap()
    );
    payload
  }
}

impl From<&NotificationMessage> for DiscordWebHookBody {
  fn from(event: &NotificationMessage) -> Self {
    Self::new(event)
  }
}

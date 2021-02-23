use crate::notifications::NotificationMessage;
use inflections::case::to_title_case;
use log::debug;
use serde::{Deserialize, Serialize};
use std::env;

const SUCCESS_COLOR: i32 = 4961603;
const FAILURE_COLOR: i32 = 16388413;
const GENERIC_COLOR: i32 = 32614;
const DISCORD_WEBHOOK_BASE: &str = "https://discord.com/api/webhooks";

pub fn is_discord_webhook(webhook_url: &str) -> bool {
  webhook_url.starts_with(DISCORD_WEBHOOK_BASE)
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

pub fn build_discord_payload(event: NotificationMessage) -> DiscordWebHookBody {
  let server_name = env::var("NAME").unwrap_or_else(|_| String::from("Your Valheim Server"));
  let name = event.event_type.to_string();
  let color = if name.contains("Successful") {
    SUCCESS_COLOR
  } else if name.contains("Failure") {
    FAILURE_COLOR
  } else {
    GENERIC_COLOR
  };
  let payload = DiscordWebHookBody {
    content: to_title_case(format!("Notification From: {}", server_name).as_str()),
    embeds: vec![DiscordWebHookEmbed {
      title: name,
      description: event.event_message,
      color,
    }],
  };
  debug!(
    "Discord Payload: {}",
    serde_json::to_string(&payload).unwrap()
  );
  payload
}

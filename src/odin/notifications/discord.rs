use crate::{files::discord::load_discord, notifications::EventStatus};

use crate::notifications::{get_notification_server_name, NotificationMessage};
use handlebars::Handlebars;
use log::debug;
use serde::{Deserialize, Serialize};

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
  pub(crate) title: String,
  pub(crate) description: String,
  pub(crate) color: i32,
}

#[derive(Deserialize, Serialize)]
pub struct DiscordWebHookBody {
  pub(crate) content: String,
  pub(crate) embeds: Vec<DiscordWebHookEmbed>,
}

pub fn body_template() -> DiscordWebHookBody {
  DiscordWebHookBody {
    content: "Notification: {{server_name}}".to_string(),
    embeds: vec![DiscordWebHookEmbed {
      title: "{{title}}".to_string(),
      description: "{{description}}".to_string(),
      color: 16388413,
    }],
  }
}

#[derive(Deserialize, Serialize)]
pub struct IncomingNotification {
  title: String,
  description: String,
  status: String,
  timestamp: String,
  server_name: String,
}

impl From<&NotificationMessage> for IncomingNotification {
  fn from(notification: &NotificationMessage) -> IncomingNotification {
    IncomingNotification {
      title: String::from(&notification.event_type.name),
      description: String::from(&notification.event_message),
      status: String::from(&notification.event_type.status),
      timestamp: String::from(&notification.timestamp),
      server_name: get_notification_server_name(),
    }
  }
}

impl From<&NotificationMessage> for DiscordWebHookBody {
  fn from(event: &NotificationMessage) -> Self {
    let discord_file = load_discord();
    let mut handlebars = Handlebars::new();
    let default_event = body_template();
    let discord_event = &discord_file
      .events
      .get(&event.event_type.name.as_str().to_lowercase())
      .unwrap_or(&default_event);
    let source = serde_json::to_string(&discord_event).unwrap();
    debug!("Discord Notification Template: {}", &source);
    handlebars
      .register_template_string("notification", source)
      .unwrap();

    let values = IncomingNotification::from(event);
    debug!(
      "Discord Notification Values: {}",
      serde_json::to_string(&values).unwrap()
    );
    let rendered = match handlebars.render("notification", &values) {
      Ok(value) => {
        debug!("Discord Notification Parsed: \n{}", value);
        value
      }
      Err(msg) => panic!("{}", msg.to_string()),
    };
    serde_json::from_str(&rendered).unwrap()
  }
}

use crate::files::discord::load_discord;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::parse_server_name_for_notification;
use crate::notifications::enums::player::PlayerStatus;
use crate::notifications::NotificationMessage;
use handlebars::Handlebars;
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
enum Color {
  Success = 0x4B_B5_43,
  Failure = 0xFA_11_3D,
  Generic = 0x00_7F_66,
  Join = 0x34_98_DB,
  Leave = 0xE7_4C_3C,
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

impl From<PlayerStatus> for Color {
  fn from(status: PlayerStatus) -> Self {
    use PlayerStatus::{Joined, Left};
    match status {
      Joined => Self::Join,
      Left => Self::Leave,
    }
  }
}

pub fn is_discord_webhook(webhook_url: &str) -> bool {
  webhook_url.starts_with(DISCORD_WEBHOOK_BASE) || webhook_url.starts_with(DISCORDAPP_WEBHOOK_BASE)
}

fn determine_color_from_notification(notification: &NotificationMessage) -> Color {
  // First try to determine color based on status
  match notification.event_type.status.as_str() {
    "Successful" => Color::Success,
    "Failed" => Color::Failure,
    "Running" => Color::Generic,
    // For player events, the status contains the player action
    "Joined" => Color::Join,
    "Left" => Color::Leave,
    _ => Color::Generic,
  }
}
#[derive(Deserialize, Serialize)]
pub struct DiscordWebHookEmbed {
  pub(crate) title: String,
  pub(crate) description: String,
  pub(crate) color: i32,
}

impl Clone for DiscordWebHookEmbed {
  fn clone(&self) -> Self {
    DiscordWebHookEmbed {
      title: String::from(&self.title),
      description: String::from(&self.description),
      color: self.color,
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct DiscordWebHookBody {
  pub(crate) content: String,
  pub(crate) embeds: Vec<DiscordWebHookEmbed>,
}

impl Clone for DiscordWebHookBody {
  fn clone(&self) -> Self {
    DiscordWebHookBody {
      content: String::from(&self.content),
      embeds: self.embeds.clone(),
    }
  }
}

impl Default for DiscordWebHookBody {
  fn default() -> Self {
    DiscordWebHookBody {
      content: "Notification: {{server_name}}".to_string(),
      embeds: vec![DiscordWebHookEmbed {
        title: "{{title}}".to_string(),
        description: "{{description}}".to_string(),
        color: Color::Generic as i32,
      }],
    }
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
      server_name: parse_server_name_for_notification(),
    }
  }
}

impl From<&NotificationMessage> for DiscordWebHookBody {
  fn from(event: &NotificationMessage) -> Self {
    let discord_file = load_discord();
    let mut handlebars = Handlebars::new();
    let default_event = DiscordWebHookBody::default();
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

    let mut discord_body: DiscordWebHookBody = serde_json::from_str(&rendered).unwrap();

    // Apply appropriate color based on the event
    let color = determine_color_from_notification(event) as i32;
    for embed in &mut discord_body.embeds {
      embed.color = color;
    }

    discord_body
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::notifications::enums::event_status::EventStatus;
  use crate::notifications::enums::notification_event::NotificationEvent;
  use crate::notifications::enums::player::PlayerStatus;
  use crate::notifications::NotificationMessage;
  use chrono::Local;

  #[test]
  fn test_color_from_event_status() {
    assert_eq!(Color::from(EventStatus::Successful), Color::Success);
    assert_eq!(Color::from(EventStatus::Failed), Color::Failure);
    assert_eq!(Color::from(EventStatus::Running), Color::Generic);
  }

  #[test]
  fn test_color_from_player_status() {
    assert_eq!(Color::from(PlayerStatus::Joined), Color::Join);
    assert_eq!(Color::from(PlayerStatus::Left), Color::Leave);
  }

  #[test]
  fn test_body_template() {
    let template = DiscordWebHookBody::default();
    assert_eq!(template.content, "Notification: {{server_name}}");
    assert_eq!(template.embeds.len(), 1);
    assert_eq!(template.embeds[0].title, "{{title}}");
    assert_eq!(template.embeds[0].description, "{{description}}");
    assert_eq!(template.embeds[0].color, Color::Generic as i32);
  }

  #[test]
  fn test_discord_webhook_body_from_notification_message() {
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Player(PlayerStatus::Joined).to_event_type(),
      event_message: String::from("Player has joined the game."),
      timestamp: Local::now().to_rfc3339(),
    };

    let discord_body: DiscordWebHookBody = (&notification).into();
    assert_eq!(discord_body.embeds.len(), 1);
    assert_eq!(discord_body.embeds[0].title, "Player");
    assert_eq!(
      discord_body.embeds[0].description,
      "Player has joined the game."
    );
  }

  #[test]
  fn test_color_application_for_player_join() {
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Player(PlayerStatus::Joined).to_event_type(),
      event_message: String::from("Player has joined the game."),
      timestamp: Local::now().to_rfc3339(),
    };

    let discord_body: DiscordWebHookBody = (&notification).into();
    assert_eq!(discord_body.embeds[0].color, Color::Join as i32);
  }

  #[test]
  fn test_color_application_for_successful_status() {
    let mut event_type = NotificationEvent::Start(EventStatus::Successful).to_event_type();
    event_type.status = "Successful".to_string();

    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type,
      event_message: String::from("Server started successfully."),
      timestamp: Local::now().to_rfc3339(),
    };

    let discord_body: DiscordWebHookBody = (&notification).into();
    assert_eq!(discord_body.embeds[0].color, Color::Success as i32);
  }
}

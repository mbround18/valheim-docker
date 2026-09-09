use crate::files::discord::load_discord;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::parse_server_name_for_notification;
use crate::notifications::enums::player::PlayerStatus;
use crate::notifications::NotificationMessage;
use crate::utils::environment::is_env_var_truthy_with_default;
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

/// Discord's `SUPPRESS_NOTIFICATIONS` message flag (`1 << 12`).
///
/// A message sent with this flag still appears in the channel, but does not
/// push a notification to its members.
pub(crate) const SUPPRESS_NOTIFICATIONS: i32 = 4096;

/// Set to `1` to send every Discord notification silently.
pub const WEBHOOK_SUPPRESS_NOTIFICATIONS: &str = "WEBHOOK_SUPPRESS_NOTIFICATIONS";

/// Whether Discord messages should be delivered without pushing a notification.
fn should_suppress_notifications() -> bool {
  is_env_var_truthy_with_default(WEBHOOK_SUPPRESS_NOTIFICATIONS, false)
}

/// Add `SUPPRESS_NOTIFICATIONS` to any flags the template already set.
///
/// Flags are a bitfield, so this ORs rather than replaces: a template opting a
/// single event into another flag keeps it when suppression is switched on
/// globally.
fn with_suppress_flag(existing: Option<i32>) -> Option<i32> {
  Some(existing.unwrap_or(0) | SUPPRESS_NOTIFICATIONS)
}

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
  /// Discord message flags. Omitted from the payload entirely when unset, so an
  /// existing `discord.json` without this key round-trips unchanged.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) flags: Option<i32>,
}

impl Clone for DiscordWebHookBody {
  fn clone(&self) -> Self {
    DiscordWebHookBody {
      content: String::from(&self.content),
      embeds: self.embeds.clone(),
      flags: self.flags,
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
      flags: None,
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
    debug!("Discord Notification Template: {}", source);
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

    // Suppression is additive: a template may already set flags for a single
    // event, and the env var turns it on for every event.
    if should_suppress_notifications() {
      discord_body.flags = with_suppress_flag(discord_body.flags);
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
  use serial_test::serial;
  use std::env::{remove_var, set_var};

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
  #[serial]
  fn test_discord_webhook_body_from_notification_message() {
    set_var(
      "NAME",
      "test_discord_webhook_body_from_notification_message",
    );
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
  #[serial]
  fn test_color_application_for_player_join() {
    set_var("NAME", "test_color_application_for_player_join");
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Player(PlayerStatus::Joined).to_event_type(),
      event_message: String::from("Player has joined the game."),
      timestamp: Local::now().to_rfc3339(),
    };

    let discord_body: DiscordWebHookBody = (&notification).into();
    assert_eq!(discord_body.embeds[0].color, Color::Join as i32);
  }

  fn player_join_notification(name: &str) -> NotificationMessage {
    set_var("NAME", name);
    NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Player(PlayerStatus::Joined).to_event_type(),
      event_message: String::from("Player has joined the game."),
      timestamp: Local::now().to_rfc3339(),
    }
  }

  #[test]
  #[serial]
  fn test_flags_absent_by_default() {
    remove_var(WEBHOOK_SUPPRESS_NOTIFICATIONS);
    let body: DiscordWebHookBody =
      (&player_join_notification("test_flags_absent_by_default")).into();
    assert_eq!(body.flags, None);
  }

  #[test]
  #[serial]
  fn test_flags_omitted_from_payload_when_unset() {
    // Discord rejects unknown/null keys less gracefully than missing ones, and
    // existing discord.json files have no flags key at all.
    remove_var(WEBHOOK_SUPPRESS_NOTIFICATIONS);
    let body: DiscordWebHookBody = (&player_join_notification("omitted-payload-server")).into();
    let payload = serde_json::to_string(&body).unwrap();
    assert!(
      !payload.contains("\"flags\""),
      "unset flags must not appear in the payload: {payload}"
    );
  }

  #[test]
  #[serial]
  fn test_suppress_notifications_sets_flag() {
    set_var(WEBHOOK_SUPPRESS_NOTIFICATIONS, "1");
    let body: DiscordWebHookBody =
      (&player_join_notification("test_suppress_notifications_sets_flag")).into();
    remove_var(WEBHOOK_SUPPRESS_NOTIFICATIONS);

    assert_eq!(body.flags, Some(4096));
    let payload = serde_json::to_string(&body).unwrap();
    assert!(
      payload.contains("\"flags\":4096"),
      "payload should carry the suppress flag: {payload}"
    );
  }

  #[test]
  #[serial]
  fn test_suppress_notifications_off_leaves_flags_unset() {
    set_var(WEBHOOK_SUPPRESS_NOTIFICATIONS, "0");
    let body: DiscordWebHookBody =
      (&player_join_notification("test_suppress_notifications_off_leaves_flags_unset")).into();
    remove_var(WEBHOOK_SUPPRESS_NOTIFICATIONS);
    assert_eq!(body.flags, None);
  }

  #[test]
  fn test_suppress_flag_matches_discord_value() {
    // Discord defines SUPPRESS_NOTIFICATIONS as 1 << 12.
    assert_eq!(SUPPRESS_NOTIFICATIONS, 1 << 12);
    assert_eq!(SUPPRESS_NOTIFICATIONS, 4096);
  }

  #[test]
  fn test_with_suppress_flag_sets_flag_when_none() {
    assert_eq!(with_suppress_flag(None), Some(SUPPRESS_NOTIFICATIONS));
  }

  #[test]
  fn test_with_suppress_flag_is_additive_with_template_flags() {
    // A template may already set a flag such as SUPPRESS_EMBEDS (4); turning on
    // suppression must add to it rather than replace it.
    const SUPPRESS_EMBEDS: i32 = 4;
    let combined = with_suppress_flag(Some(SUPPRESS_EMBEDS)).unwrap();
    assert_eq!(combined, 4100);
    assert_eq!(combined & SUPPRESS_EMBEDS, SUPPRESS_EMBEDS);
    assert_eq!(combined & SUPPRESS_NOTIFICATIONS, SUPPRESS_NOTIFICATIONS);
  }

  #[test]
  fn test_with_suppress_flag_is_idempotent() {
    let once = with_suppress_flag(None);
    assert_eq!(with_suppress_flag(once), once);
  }

  #[test]
  fn test_body_deserializes_without_flags_key() {
    // Existing discord.json files predate the flags field.
    let json = r#"{"content":"c","embeds":[{"title":"t","description":"d","color":1}]}"#;
    let body: DiscordWebHookBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.flags, None);
    assert_eq!(body.content, "c");
  }

  #[test]
  fn test_body_deserializes_with_flags_key() {
    // Per-event opt-in directly in discord.json.
    let json =
      r#"{"content":"c","embeds":[{"title":"t","description":"d","color":1}],"flags":4096}"#;
    let body: DiscordWebHookBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.flags, Some(4096));
  }

  #[test]
  fn test_clone_preserves_flags() {
    let body = DiscordWebHookBody {
      flags: Some(SUPPRESS_NOTIFICATIONS),
      ..Default::default()
    };
    assert_eq!(body.clone().flags, Some(SUPPRESS_NOTIFICATIONS));
  }

  #[test]
  #[serial]
  fn test_color_application_for_successful_status() {
    set_var("NAME", "test_color_application_for_successful_status");
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

use crate::files::discord::load_discord;
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::parse_server_name_for_notification;
use crate::notifications::enums::player::PlayerStatus;
use crate::notifications::NotificationMessage;
use crate::utils::environment::{fetch_var, is_env_var_truthy_with_default};
use handlebars::Handlebars;
use log::{debug, warn};
use reqwest::Url;
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

/// Set to an `https://` URL to add a "Join Server" button to start notifications.
///
/// Blank (the default) means no button. Discord rejects `steam://` in a button
/// URL, so this must point at something reachable over HTTPS -- typically a
/// reverse-proxied Huginn `/connect/remote`, which redirects on to the
/// `steam://` handoff, or any redirect service the operator prefers.
pub const WEBHOOK_JOIN_URL: &str = "WEBHOOK_JOIN_URL";

/// Discord component type for an action row.
const COMPONENT_TYPE_ACTION_ROW: u8 = 1;
/// Discord component type for a button.
const COMPONENT_TYPE_BUTTON: u8 = 2;
/// Discord button style for a link button (no interaction is sent to any app).
const BUTTON_STYLE_LINK: u8 = 5;

const JOIN_BUTTON_LABEL: &str = "Join Server";

#[derive(Clone, Deserialize, Serialize)]
pub struct DiscordButton {
  #[serde(rename = "type")]
  pub(crate) component_type: u8,
  pub(crate) style: u8,
  pub(crate) label: String,
  pub(crate) url: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DiscordActionRow {
  #[serde(rename = "type")]
  pub(crate) component_type: u8,
  pub(crate) components: Vec<DiscordButton>,
}

/// The configured join URL, if it is set and usable.
///
/// Discord requires an `https://` (or `http://`) scheme on link buttons and
/// rejects anything else with a 400. We require HTTPS specifically: the link is
/// posted publicly in a channel, and an operator standing up a redirect service
/// can serve it over TLS.
fn join_button_url() -> Option<String> {
  let configured = fetch_var(WEBHOOK_JOIN_URL, "");
  let trimmed = configured.trim().trim_matches('"').trim();
  if trimmed.is_empty() {
    return None;
  }

  match Url::parse(trimmed) {
    Ok(parsed) if parsed.scheme() == "https" => Some(trimmed.to_string()),
    Ok(parsed) => {
      warn!(
        "{WEBHOOK_JOIN_URL} must use https://, got '{}://'. Skipping join button.",
        parsed.scheme()
      );
      None
    }
    Err(e) => {
      warn!("{WEBHOOK_JOIN_URL} is not a valid URL ({e}). Skipping join button.");
      None
    }
  }
}

/// Whether this event is the one worth attaching a join button to.
///
/// Only a successful start means the server is actually up and joinable. A
/// button on a stop, a failure, or a player-left event would be misleading.
fn event_takes_join_button(event: &NotificationMessage) -> bool {
  event.event_type.name.eq_ignore_ascii_case("start")
    && event.event_type.status.eq_ignore_ascii_case("successful")
}

/// Build the single-button action row linking players at the join URL.
fn join_button_components(url: String) -> Vec<DiscordActionRow> {
  vec![DiscordActionRow {
    component_type: COMPONENT_TYPE_ACTION_ROW,
    components: vec![DiscordButton {
      component_type: COMPONENT_TYPE_BUTTON,
      style: BUTTON_STYLE_LINK,
      label: String::from(JOIN_BUTTON_LABEL),
      url,
    }],
  }]
}

/// Add `with_components=true` to a webhook URL.
///
/// A webhook that is not application-owned silently drops `components` unless
/// this query parameter is present -- Discord returns a normal 204 with the
/// button missing rather than an error. Returns the URL unchanged if it cannot
/// be parsed, leaving the send to fail visibly rather than here.
pub fn with_components_query(webhook_url: &str) -> String {
  match Url::parse(webhook_url) {
    Ok(mut parsed) => {
      parsed
        .query_pairs_mut()
        .append_pair("with_components", "true");
      parsed.to_string()
    }
    Err(e) => {
      warn!("Could not add with_components to webhook URL ({e}); sending as-is.");
      webhook_url.to_string()
    }
  }
}

/// Escape a value for interpolation into a JSON string literal.
///
/// Handlebars defaults to HTML escaping, but this template renders JSON, not
/// markup. HTML escaping turned any `=` in a message into `&#x3D;`, and `&`,
/// `<`, `>`, `"` and `'` likewise, so the text arrived in Discord mangled.
///
/// Escaping cannot simply be disabled: the rendered string is parsed back with
/// `serde_json`, so a message containing a quote, a backslash or a newline
/// would produce invalid JSON. Serializing the value as a JSON string and
/// trimming its surrounding quotes escapes exactly what JSON requires and
/// nothing else.
fn json_escape(data: &str) -> String {
  match serde_json::to_string(data) {
    // to_string on a &str always yields a quoted string, so the slice is safe.
    Ok(encoded) => encoded[1..encoded.len() - 1].to_string(),
    Err(_) => String::new(),
  }
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
  /// Message components (the join button). Omitted when there are none, and
  /// never present in a user's `discord.json`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) components: Option<Vec<DiscordActionRow>>,
}

impl Clone for DiscordWebHookBody {
  fn clone(&self) -> Self {
    DiscordWebHookBody {
      content: String::from(&self.content),
      embeds: self.embeds.clone(),
      flags: self.flags,
      components: self.components.clone(),
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
      components: None,
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
    // The template is JSON; escape for that, not for HTML.
    handlebars.register_escape_fn(json_escape);
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

    // Only a successful start gets a join button, and only when an operator has
    // supplied a URL for it.
    if event_takes_join_button(event) {
      if let Some(url) = join_button_url() {
        debug!(
          "Attaching join button to {} notification",
          event.event_type.name
        );
        discord_body.components = Some(join_button_components(url));
      }
    }

    discord_body
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::notifications::enums::event_status::EventStatus;
  use crate::notifications::enums::notification_event::{EventType, NotificationEvent};
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

  fn start_successful(name: &str) -> NotificationMessage {
    set_var("NAME", name);
    NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Start(EventStatus::Successful).to_event_type(),
      event_message: String::from("Server Status: Start Successful"),
      timestamp: Local::now().to_rfc3339(),
    }
  }

  // --- join button: URL validation ---

  #[test]
  #[serial]
  fn test_join_url_blank_by_default() {
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(join_button_url(), None);
  }

  #[test]
  #[serial]
  fn test_join_url_empty_string_is_no_button() {
    set_var(WEBHOOK_JOIN_URL, "   ");
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(got, None);
  }

  #[test]
  #[serial]
  fn test_join_url_accepts_https() {
    set_var(
      WEBHOOK_JOIN_URL,
      "https://valheim.example.com/connect/remote",
    );
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(
      got,
      Some(String::from("https://valheim.example.com/connect/remote"))
    );
  }

  #[test]
  #[serial]
  fn test_join_url_strips_surrounding_quotes() {
    // Compose files routinely quote values.
    set_var(WEBHOOK_JOIN_URL, "\"https://example.com/join\"");
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(got, Some(String::from("https://example.com/join")));
  }

  #[test]
  #[serial]
  fn test_join_url_rejects_plain_http() {
    set_var(WEBHOOK_JOIN_URL, "http://example.com/join");
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(got, None);
  }

  #[test]
  #[serial]
  fn test_join_url_rejects_steam_scheme() {
    // Discord answers a steam:// button URL with a 400, so it must never reach
    // the API.
    set_var(WEBHOOK_JOIN_URL, "steam://connect/1.2.3.4:2456");
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(got, None);
  }

  #[test]
  #[serial]
  fn test_join_url_rejects_garbage() {
    set_var(WEBHOOK_JOIN_URL, "not a url");
    let got = join_button_url();
    remove_var(WEBHOOK_JOIN_URL);
    assert_eq!(got, None);
  }

  // --- join button: which events get one ---

  #[test]
  fn test_only_successful_start_takes_a_join_button() {
    let cases = [
      ("Start", "Successful", true),
      ("Start", "Running", false),
      ("Start", "Failed", false),
      ("Stop", "Successful", false),
      ("Update", "Successful", false),
      ("Player", "Joined", false),
      ("Broadcast", "Triggered", false),
    ];
    for (name, status, expected) in cases {
      let event = NotificationMessage {
        author: String::from("a"),
        event_type: EventType {
          name: String::from(name),
          status: String::from(status),
        },
        event_message: String::from("m"),
        timestamp: String::from("t"),
      };
      assert_eq!(
        event_takes_join_button(&event),
        expected,
        "{name} {status} should{} take a join button",
        if expected { "" } else { " not" }
      );
    }
  }

  // --- join button: payload shape ---

  #[test]
  #[serial]
  fn test_no_components_when_join_url_unset() {
    remove_var(WEBHOOK_JOIN_URL);
    let body: DiscordWebHookBody = (&start_successful("no-button-server")).into();
    assert!(body.components.is_none());
    let payload = serde_json::to_string(&body).unwrap();
    assert!(
      !payload.contains("components"),
      "components must be omitted entirely: {payload}"
    );
  }

  #[test]
  #[serial]
  fn test_start_success_carries_join_button() {
    set_var(WEBHOOK_JOIN_URL, "https://example.com/join");
    let body: DiscordWebHookBody = (&start_successful("button-server")).into();
    remove_var(WEBHOOK_JOIN_URL);

    let rows = body.components.expect("expected components");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].component_type, COMPONENT_TYPE_ACTION_ROW);
    assert_eq!(rows[0].components.len(), 1);

    let button = &rows[0].components[0];
    assert_eq!(button.component_type, COMPONENT_TYPE_BUTTON);
    assert_eq!(button.style, BUTTON_STYLE_LINK);
    assert_eq!(button.label, JOIN_BUTTON_LABEL);
    assert_eq!(button.url, "https://example.com/join");
  }

  #[test]
  #[serial]
  fn test_join_button_serializes_to_discords_shape() {
    // Discord is strict about these numbers: action row 1, button 2, link 5.
    set_var(WEBHOOK_JOIN_URL, "https://example.com/join");
    let body: DiscordWebHookBody = (&start_successful("shape-server")).into();
    remove_var(WEBHOOK_JOIN_URL);

    let payload = serde_json::to_value(&body).unwrap();
    let row = &payload["components"][0];
    assert_eq!(row["type"], 1);
    assert_eq!(row["components"][0]["type"], 2);
    assert_eq!(row["components"][0]["style"], 5);
    assert_eq!(row["components"][0]["url"], "https://example.com/join");
    assert_eq!(row["components"][0]["label"], "Join Server");
  }

  #[test]
  #[serial]
  fn test_non_start_event_has_no_button_even_with_url_set() {
    set_var(WEBHOOK_JOIN_URL, "https://example.com/join");
    set_var("NAME", "player-event-server");
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Player(PlayerStatus::Joined).to_event_type(),
      event_message: String::from("Player has joined the game."),
      timestamp: Local::now().to_rfc3339(),
    };
    let body: DiscordWebHookBody = (&notification).into();
    remove_var(WEBHOOK_JOIN_URL);
    assert!(body.components.is_none());
  }

  // --- with_components query parameter ---

  #[test]
  fn test_with_components_query_appends_param() {
    let url = with_components_query("https://discord.com/api/webhooks/1/tok");
    assert_eq!(
      url,
      "https://discord.com/api/webhooks/1/tok?with_components=true"
    );
  }

  #[test]
  fn test_with_components_query_preserves_existing_query() {
    let url = with_components_query("https://discord.com/api/webhooks/1/tok?wait=true");
    assert!(url.contains("wait=true"), "{url}");
    assert!(url.contains("with_components=true"), "{url}");
  }

  #[test]
  fn test_with_components_query_returns_unparseable_url_unchanged() {
    assert_eq!(with_components_query("not a url"), "not a url");
  }

  #[test]
  fn test_json_escape_leaves_plain_punctuation_alone() {
    // Regression: HTML escaping rendered "mode=0" as "mode&#x3D;0" in Discord.
    assert_eq!(
      json_escape("flag verification mode=0"),
      "flag verification mode=0"
    );
    assert_eq!(json_escape("Bob & Alice's server"), "Bob & Alice's server");
    assert_eq!(json_escape("<Ragnarok>"), "<Ragnarok>");
  }

  #[test]
  fn test_json_escape_escapes_what_json_requires() {
    assert_eq!(json_escape("say \"hi\""), "say \\\"hi\\\"");
    assert_eq!(json_escape("back\\slash"), "back\\\\slash");
    assert_eq!(json_escape("line\nbreak"), "line\\nbreak");
    assert_eq!(json_escape("tab\there"), "tab\\there");
  }

  #[test]
  #[serial]
  fn test_message_with_equals_survives_rendering() {
    set_var("NAME", "equals-server");
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Broadcast.to_event_type(),
      event_message: String::from("flag verification mode=0"),
      timestamp: Local::now().to_rfc3339(),
    };

    let body: DiscordWebHookBody = (&notification).into();
    assert_eq!(body.embeds[0].description, "flag verification mode=0");
    assert!(!body.embeds[0].description.contains("&#x3D;"));
  }

  #[test]
  #[serial]
  fn test_message_with_quotes_still_renders_valid_json() {
    // Disabling escaping outright would break the JSON parse on this input.
    set_var("NAME", "quote-server");
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Broadcast.to_event_type(),
      event_message: String::from("player said \"hello\" & left\nbye"),
      timestamp: Local::now().to_rfc3339(),
    };

    let body: DiscordWebHookBody = (&notification).into();
    assert_eq!(
      body.embeds[0].description,
      "player said \"hello\" & left\nbye"
    );
  }

  #[test]
  #[serial]
  fn test_server_name_with_ampersand_is_not_html_escaped() {
    set_var("NAME", "Bob & Alice's Realm");
    let notification = NotificationMessage {
      author: String::from("Test Author"),
      event_type: NotificationEvent::Broadcast.to_event_type(),
      event_message: String::from("hello"),
      timestamp: Local::now().to_rfc3339(),
    };

    let body: DiscordWebHookBody = (&notification).into();
    assert!(
      body.content.contains("Bob & Alice's Realm"),
      "server name should not be HTML escaped: {}",
      body.content
    );
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

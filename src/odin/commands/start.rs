use crate::{
  files::config::load_config,
  notifications::enums::{event_status::EventStatus, notification_event::NotificationEvent},
  server,
};

use log::{debug, error, info};
use std::process::exit;

pub fn invoke(dry_run: bool) {
  info!(target: "commands_start", "Setting up start scripts...");
  NotificationEvent::Start(EventStatus::Running).send_notification();
  debug!(target: "commands_start", "Loading config file...");
  let config = load_config();
  debug!(target: "commands_start", "Dry run condition: {}", dry_run);
  info!(target: "commands_start", "Looking for burial mounds...");
  if !dry_run {
    match server::start_daemonized(config) {
      Ok(_) => info!(target: "commands_start", "Success, daemonized"),
      Err(e) => {
        error!(target: "commands_start", "Error: {}", e);
        exit(1);
      }
    }
  } else {
    info!(
      target: "commands_start",
      "This command would have launched\n{} -nographics -batchmode -port {} -name {} -world {} -password {} -public {}",
      &config.command,
      &config.port,
      &config.name,
      &config.world,
      &config.password,
      &config.public,
    )
  }
}

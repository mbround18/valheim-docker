use log::{error, info};

use std::process::exit;

use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::NotificationEvent;
use crate::{constants, server, utils::get_working_dir};

pub fn invoke(dry_run: bool) {
  NotificationEvent::Stop(EventStatus::Running).send_notification();
  info!("Stopping server {}", get_working_dir());
  if dry_run {
    info!("This command would have run: ");
    info!("kill -2 {}", constants::VALHEIM_EXECUTABLE_NAME)
  } else {
    if !server::is_installed() {
      error!("Failed to find server executable!");
      exit(1);
    }
    server::blocking_shutdown();
  }
  NotificationEvent::Stop(EventStatus::Successful).send_notification();
}

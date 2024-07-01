use crate::{
  files::config::{self, load_config},
  notifications::enums::{event_status::EventStatus, notification_event::NotificationEvent},
  server,
};

use log::{debug, error, info};
use std::process::{exit, Child};

fn run_server(config: &config::ValheimArguments) -> Child {
  server::start_daemonized(config)
    .map_err(|err| format!("{}", err))
    .and_then(|it| it.map_err(|err| format!("{}", err)))
    .map_err(|err: String| {
      error!(target: "commands_start", "Error: {}", err);
      exit(1);
    })
    .expect("failed to daemonize server")
}

pub fn invoke(dry_run: bool, pause_on_idle_s: u32) {
  info!(target: "commands_start", "Setting up start scripts...");
  NotificationEvent::Start(EventStatus::Running).send_notification();
  debug!(target: "commands_start", "Loading config file...");
  let config = load_config();
  debug!(target: "commands_start", "Dry run condition: {}", dry_run);
  info!(target: "commands_start", "Looking for burial mounds ZZZ...");
  if !dry_run {
    match pause_on_idle_s > 0 {
      true => {
        info!(target: "commands_start", "Starting server with idle pausing ({} seconds)", pause_on_idle_s);
        let mut child = run_server(&config);
        let _ = server::handle_idle(&mut child, pause_on_idle_s, &config.port).map_err(|err| {
          println!("failed to monitor game process: ${}", &err);
          exit(1);
        });
      }
      false => {
        run_server(&config)
          .wait()
          .expect("server exited unexpectedly");
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

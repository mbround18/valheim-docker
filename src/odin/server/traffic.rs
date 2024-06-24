use log::info;
use nix::{sys::signal, unistd::Pid};
use pcap::{Capture, Device};
use std::process::Child;

enum TrafficState {
  Running,
  Paused,
}

pub fn handle_idle(
  child: &mut Child,
  pause_on_idle_s: u32,
  server_port: &str,
) -> Result<(), String> {
  let device = Device::lookup()
    .map_err(|it| format!("{}", it))?
    .ok_or("No devices found".to_string())?;
  let mut cap = Capture::from_device(device)
    .map_err(|it| it.to_string())?
    .timeout(pause_on_idle_s.try_into().map_err(|it| format!("{}", it))?)
    .promisc(true)
    .open()
    .map_err(|it| it.to_string())?;
  cap
    .filter(&format!("udp port {}", server_port), true)
    .map_err(|it| it.to_string())?;

  let mut state = TrafficState::Running;
  loop {
    match (cap.next_packet(), &state) {
      (Ok(_), TrafficState::Running) => {
        // pass - packet was received, so we are not idle
      }
      (Ok(_), TrafficState::Paused) => {
        info!("Traffic detected, resuming server");
        signal::kill(Pid::from_raw(child.id() as i32), nix::sys::signal::SIGCONT)
          .map_err(|it| format!("{}", it))?;
        state = TrafficState::Running;
      }
      (Err(pcap::Error::TimeoutExpired), TrafficState::Running) => {
        info!("No traffic detected, pausing server");
        nix::sys::signal::kill(Pid::from_raw(child.id() as i32), nix::sys::signal::SIGSTOP)
          .map_err(|it| format!("{}", it))?;
        state = TrafficState::Paused;
      }
      (Err(pcap::Error::TimeoutExpired), TrafficState::Paused) => {
        // pass - we are already paused
      }
      (Err(e), _) => {
        return Err(e.to_string());
      }
    }
  }
}

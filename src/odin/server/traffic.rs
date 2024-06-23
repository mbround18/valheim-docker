use std::process::Child;

use pcap::{Capture, Device};

pub fn spawn_kill_on_idle(child: &mut Child, pause_on_idle_s: u32) -> Result<(), String> {
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
    // @todo uh errr uhhh get the actual server port into here
    .filter(&format!("udp port {}", "2456"), true)
    .map_err(|it| it.to_string())?;

  while let Ok(_packet) = cap.next_packet() {
    // pass - packet was received, so we are not idle
  }
  child.kill().map_err(|e| e.to_string())?;
  Ok(())
}

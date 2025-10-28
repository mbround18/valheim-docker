use crate::fetch_info;
use shared::system::collect_system_metrics;

pub fn invoke() -> String {
  let info = fetch_info();
  let sys = collect_system_metrics();
  let labels = format!(
    "{{name=\"{name}\", version=\"{version}\", map=\"{map}\"}}",
    name = &info.name,
    version = &info.version,
    map = &info.map
  );
  let content = [
    format!(
      "valheim_online{labels} {online}",
      labels = &labels,
      online = info.online as i32
    ),
    format!(
      "valheim_current_player_count{labels} {players}",
      labels = &labels,
      players = &info.players
    ),
    format!(
      "valheim_max_player_count{labels} {players}",
      labels = &labels,
      players = &info.max_players
    ),
    format!(
      "valheim_bepinex_installed{labels} {bepinex_installed}",
      labels = &labels,
      bepinex_installed = info.bepinex.enabled as i32
    ),
    // System metrics (no labels beyond server identity)
    format!(
      "valheim_sys_memory_total_bytes {:.0}",
      sys.total_memory_bytes
    ),
    format!("valheim_sys_memory_used_bytes {:.0}", sys.used_memory_bytes),
    format!("valheim_sys_swap_total_bytes {:.0}", sys.total_swap_bytes),
    format!("valheim_sys_swap_used_bytes {:.0}", sys.used_swap_bytes),
    format!("valheim_sys_disk_total_bytes {:.0}", sys.total_disk_bytes),
    format!(
      "valheim_sys_disk_available_bytes {:.0}",
      sys.available_disk_bytes
    ),
    format!("valheim_sys_cpu_logical_count {}", sys.cpu_num_logical),
    format!(
      "valheim_sys_load_average {{window=\"1m\"}} {:.2}",
      sys.load_average_one
    ),
    format!(
      "valheim_sys_load_average {{window=\"5m\"}} {:.2}",
      sys.load_average_five
    ),
    format!(
      "valheim_sys_load_average {{window=\"15m\"}} {:.2}",
      sys.load_average_fifteen
    ),
  ];
  format!("{}\n", content.join("\n"))
}

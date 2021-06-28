use crate::fetch_info;

pub fn invoke() -> String {
  let info = fetch_info();
  let labels = format!(
    "{{name=\"{name}\", version=\"{version}\", map=\"{map}\"}}",
    name = &info.name,
    version = &info.version,
    map = &info.map
  );
  let content = vec![
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
  ];
  format!("{}\n", content.join("\n"))
}

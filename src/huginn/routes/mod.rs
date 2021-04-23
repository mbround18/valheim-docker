use crate::fetch_info;

pub(crate) mod metrics;
pub(crate) mod status;

pub fn invoke() -> String {
  let info = fetch_info();
  let status_message = if info.online { "online" } else { "offline" };
  format!("{} is {}", &info.name, &status_message)
}

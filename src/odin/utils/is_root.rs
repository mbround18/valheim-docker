use std::env;
use users::{get_current_uid, get_user_by_uid};

pub fn is_root() -> bool {
  let bypass = env::var("ALLOW_RUN_AS_ROOT").unwrap_or(String::from("0"));
  if bypass.eq("1") || bypass.eq("true") {
    return true;
  }
  let current_uid = get_current_uid();
  match get_user_by_uid(current_uid) {
    Some(user) => user.uid() == 0,
    None => panic!("Failed to get user information."),
  }
}

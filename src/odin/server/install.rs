use log::{debug, info};

use std::{
  env, io,
  path::Path,
  process::{ExitStatus, Stdio},
};

use crate::utils::environment;
use crate::{
  constants, executable::execute_mut, steamcmd::steamcmd_command, utils::get_working_dir,
};

pub fn is_installed() -> bool {
  Path::new(&get_working_dir())
    .join(constants::VALHEIM_EXECUTABLE_NAME)
    .exists()
}

fn add_additional_args(args: &mut Vec<String>) {
  if let Ok(extra_args) = env::var("ADDITIONAL_STEAMCMD_ARGS") {
    args.push(String::from(
      extra_args.trim_start_matches('"').trim_end_matches('"'),
    ))
  }
}

pub fn install(app_id: i64) -> io::Result<ExitStatus> {
  info!("Installing {} to {}", app_id, get_working_dir());

  let login = "+login anonymous".to_string();
  let force_install_dir = format!("+force_install_dir {}", get_working_dir());
  let app_update = format!("+app_update {}", app_id);
  let mut steamcmd = steamcmd_command();
  let mut args = vec![force_install_dir, login, app_update];
  // Option to have steamcmd be verbose
  if environment::fetch_var("DEBUG_MODE", "0").eq("1") {
    args.push(String::from("+verbose"));
  }
  add_additional_args(&mut args);

  let install_command = steamcmd
    .args(&args)
    .arg("+quit")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
  debug!("Launching install command: {:#?}", install_command);

  execute_mut(install_command)
}

#[cfg(test)]
mod tests {
  use crate::server::install::add_additional_args;

  #[test]
  fn adding_additional_args() {
    let mut args = vec!["example".to_string()];
    std::env::set_var(
      "ADDITIONAL_STEAMCMD_ARGS",
      "\"-beta publicbeta -betapassword iamsure\"",
    );
    add_additional_args(&mut args);
    assert_eq!(
      args.join(" "),
      "example -beta publicbeta -betapassword iamsure".to_string()
    );
  }
}

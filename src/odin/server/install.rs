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

const BETA_BRANCH: &str = "public-test";
const BETA_BRANCH_PASSWORD: &str = "yesimadebackups";

pub fn is_installed() -> bool {
  Path::new(&get_working_dir())
    .join(constants::VALHEIM_EXECUTABLE_NAME)
    .exists()
}

pub fn add_beta_args(args: &mut Vec<String>) {
  let use_public_beta = environment::fetch_var("USE_PUBLIC_BETA", "0").eq("1");
  let beta_branch = env::var("BETA_BRANCH").unwrap_or(BETA_BRANCH.to_string());
  let is_backwards_compatible_branch =
    !["default_preal", "default_old", "default_preml"].contains(&beta_branch.as_str());

  if is_backwards_compatible_branch || use_public_beta {
    debug!("Using {} beta branch", beta_branch);
    args.push(format!("-beta {}", beta_branch));
  }

  if use_public_beta {
    let beta_password =
      env::var("BETA_BRANCH_PASSWORD").unwrap_or(BETA_BRANCH_PASSWORD.to_string());
    if is_backwards_compatible_branch {
      args.push(format!("-betapassword {}", beta_password));
    }
  }

  args.push(String::from("validate"));
}

fn add_additional_args(args: &mut Vec<String>) {
  if let Ok(extra_args) = env::var("ADDITIONAL_STEAMCMD_ARGS") {
    let additional_args = String::from(extra_args.trim_start_matches('"').trim_end_matches('"'));
    debug!("Adding additional arguments! {}", additional_args);
    args.push(additional_args)
  }
  add_beta_args(args);
}

pub fn install(app_id: i64) -> io::Result<ExitStatus> {
  info!("Installing {} to {}", app_id, get_working_dir());
  let login = "+login anonymous".to_string();
  let force_install_dir = format!("+force_install_dir {}", get_working_dir());
  let app_update = format!("+app_update {}", app_id);
  let mut steamcmd = steamcmd_command();
  let mut args = vec![force_install_dir, login];

  if environment::fetch_var("DEBUG_MODE", "0").eq("1") {
    args.push(String::from("verbose"));
  }

  args.push(app_update);

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
  use crate::server::install::{add_additional_args, BETA_BRANCH, BETA_BRANCH_PASSWORD};

  //   #[test]
  //   fn add_custom_args() {
  //     let mut args = vec!["example".to_string()];
  //     let extra_args = "-i -am -some -extra -args";
  //     std::env::set_var("ADDITIONAL_STEAMCMD_ARGS", format!("\"{}\"", extra_args));
  //     add_additional_args(&mut args);
  //     assert_eq!(
  //       args.join(" "),
  //       format!(
  //         "example {} -beta {} -betapassword \"{}\"",
  //         extra_args, BETA_BRANCH, BETA_BRANCH_PASSWORD
  //       )
  //     );
  //     std::env::remove_var("ADDITIONAL_STEAMCMD_ARGS");
  //   }
  #[test]
  fn add_beta_args() {
    let mut args = vec!["example".to_string()];
    std::env::set_var("ADDITIONAL_STEAMCMD_ARGS", "".to_string());
    std::env::set_var("USE_PUBLIC_BETA", "1");
    add_additional_args(&mut args);
    assert_eq!(
      args.join(" "),
      format!(
        "example  -beta {} -betapassword {} validate",
        BETA_BRANCH, BETA_BRANCH_PASSWORD
      )
    );
    std::env::remove_var("USE_PUBLIC_BETA");
  }
}

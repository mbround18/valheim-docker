use log::{debug, info};

use std::{
  env, io,
  path::Path,
  process::{ExitStatus, Stdio},
};

use crate::{constants, steamcmd::steamcmd_command, utils::get_working_dir};
use crate::{
  executable::{execute_mut_wait, parse_command_args},
  utils::environment,
};

const BETA_BRANCH: &str = "public-test";
const BETA_BRANCH_PASSWORD: &str = "yesimadebackups";

pub fn is_installed() -> bool {
  Path::new(&get_working_dir())
    .join(constants::VALHEIM_EXECUTABLE_NAME)
    .exists()
}

pub fn add_beta_args(
  args: &mut Vec<String>,
  use_public_beta: bool,
  beta_branch: String,
  beta_password: String,
) {
  let is_backwards_compatible_branch =
    ["default_preal", "default_old", "default_preml"].contains(&beta_branch.as_str());

  if is_backwards_compatible_branch || use_public_beta {
    info!("Using {beta_branch} beta branch");
    args.push(format!("-beta {beta_branch}"));
  }

  if use_public_beta && !is_backwards_compatible_branch {
    args.push(format!("-betapassword {beta_password}"));
  }

  args.push(String::from("validate"));
}

fn add_additional_args(args: &mut Vec<String>) {
  if let Ok(extra_args) = env::var("ADDITIONAL_STEAMCMD_ARGS") {
    let additional_args = String::from(extra_args.trim_start_matches('"').trim_end_matches('"'));
    debug!("Adding additional arguments! {additional_args}");
    args.push(additional_args)
  }

  let use_public_beta = environment::fetch_var("USE_PUBLIC_BETA", "0").eq("1");
  let beta_branch = env::var("BETA_BRANCH").unwrap_or(BETA_BRANCH.to_string());
  let beta_password = env::var("BETA_BRANCH_PASSWORD").unwrap_or(BETA_BRANCH_PASSWORD.to_string());

  add_beta_args(args, use_public_beta, beta_branch, beta_password);
}

pub fn install(app_id: i64) -> io::Result<ExitStatus> {
  // Pre-install logging: current build and whether beta branch will be used
  let prev_build = crate::server::try_get_current_build_id();
  if let Some(build) = &prev_build {
    info!("Current build: {build}");
  } else {
    info!("Current build: unknown (manifest not found)");
  }

  let beta_branch = env::var("BETA_BRANCH").unwrap_or(BETA_BRANCH.to_string());
  let use_public_beta = environment::fetch_var("USE_PUBLIC_BETA", "0").eq("1");
  let is_backwards_compatible_branch =
    ["default_preal", "default_old", "default_preml"].contains(&beta_branch.as_str());
  let beta_in_effect = is_backwards_compatible_branch || use_public_beta;
  if beta_in_effect {
    info!("Installing using beta branch: {beta_branch}");
  } else {
    info!("Installing using default/stable branch");
  }

  info!("Installing {} to {}", app_id, get_working_dir());
  let login = "+login anonymous".to_string();
  let force_install_dir = format!("+force_install_dir {}", get_working_dir());
  let app_update = format!("+app_update {app_id}");
  let mut steamcmd = steamcmd_command();
  let mut args = vec![force_install_dir, login];

  if environment::fetch_var("DEBUG_MODE", "0").eq("1") {
    args.push(String::from("verbose"));
  }

  args.push(app_update);

  add_additional_args(&mut args);

  // remove the call to steamccmd from args to avoid duplication
  args.retain(|arg| arg != "/usr/bin/steamcmd");

  // for args if it has a space, reduce and append it so its a seperate arg
  args = flatten_args(args);
  args = parse_command_args(args);

  let install_command = steamcmd
    .args(&args)
    .arg("+quit")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
  debug!("Launching install command: {install_command:#?}");

  let result = execute_mut_wait(install_command);

  // Post-install logging: new build id
  let post_build = crate::server::try_get_current_build_id();
  match (prev_build.as_deref(), post_build.as_deref()) {
    (Some(prev), Some(post)) if prev == post => {
      info!("No change in build version: {post}");
    }
    (Some(prev), Some(post)) => {
      if beta_in_effect {
        info!("Installed update from build {prev} -> {post} (beta: {beta_branch})");
      } else {
        info!("Installed update from build {prev} -> {post} (stable)");
      }
    }
    (_, Some(post)) => {
      if beta_in_effect {
        info!("Installed to build {post} (beta: {beta_branch})");
      } else {
        info!("Installed to build {post} (stable)");
      }
    }
    _ => info!("Install complete; build id not found."),
  }

  result
}

/// Split any argument entries containing whitespace into multiple args.
/// This helps when upstream code accidentally pushes combined tokens.
pub(crate) fn flatten_args(args: Vec<String>) -> Vec<String> {
  args
    .iter()
    .flat_map(|arg| {
      arg
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<String>>()
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;
  use test_case::test_case;

  #[test_case(
    false,
    "default_beta_branch".to_string(),
    "default_beta_password".to_string(),
    vec!["validate"]
  )]
  #[test_case(
    true,
    "public-test".to_string(),
    "yesimadebackups".to_string(),
    vec!["-beta public-test", "-betapassword yesimadebackups", "validate"]
  )]
  #[test_case(
    false,
    "default_preml".to_string(),
    "default_beta_password".to_string(),
    vec!["-beta default_preml", "validate"]
  )]
  #[test_case(
    true,
    "default_old".to_string(),
    "default_beta_password".to_string(),
    vec!["-beta default_old","validate"]
  )]
  fn test_no_beta(
    use_public_beta: bool,
    beta_branch: String,
    beta_password: String,
    expected: Vec<&str>,
  ) {
    let mut args = vec![];
    add_beta_args(&mut args, use_public_beta, beta_branch, beta_password);
    assert_eq!(args, expected);
  }

  #[test]
  fn test_add_beta_args() {
    let mut args = vec!["example".to_string()];
    env::set_var("ADDITIONAL_STEAMCMD_ARGS", "");
    env::set_var("USE_PUBLIC_BETA", "1");
    add_additional_args(&mut args);
    assert_eq!(
      args.join(" "),
      format!("example  -beta {BETA_BRANCH} -betapassword {BETA_BRANCH_PASSWORD} validate")
    );
    env::remove_var("USE_PUBLIC_BETA");
  }

  #[test]
  fn test_flatten_args_splits_simple_pair() {
    let input = vec!["+login anonymous".to_string(), "+quit".to_string()];
    let out = flatten_args(input);
    assert_eq!(out, vec!["+login", "anonymous", "+quit"]);
  }

  #[test]
  fn test_flatten_args_handles_multiple_spaces() {
    let input = vec!["  +force_install_dir   /srv/valheim  ".to_string()];
    let out = flatten_args(input);
    assert_eq!(out, vec!["+force_install_dir", "/srv/valheim"]);
  }
}

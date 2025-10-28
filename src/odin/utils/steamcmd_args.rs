use log::debug;
use std::env;

use crate::utils::environment;

const BETA_BRANCH: &str = "public-test";
const BETA_BRANCH_PASSWORD: &str = "yesimadebackups";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetaConfig {
  pub use_public_beta: bool,
  pub branch: String,
  pub password: String,
}

impl BetaConfig {
  pub fn from_env() -> Self {
    Self {
      use_public_beta: environment::is_env_var_truthy_with_default("USE_PUBLIC_BETA", false),
      branch: env::var("BETA_BRANCH").unwrap_or_else(|_| BETA_BRANCH.to_string()),
      password: env::var("BETA_BRANCH_PASSWORD")
        .unwrap_or_else(|_| BETA_BRANCH_PASSWORD.to_string()),
    }
  }

  pub fn from_parts(use_public_beta: bool, branch: String, password: String) -> Self {
    Self {
      use_public_beta,
      branch,
      password,
    }
  }

  pub fn is_backwards_compatible_branch(&self) -> bool {
    matches!(
      self.branch.as_str(),
      "default_preal" | "default_old" | "default_preml"
    )
  }

  pub fn beta_in_effect(&self) -> bool {
    self.use_public_beta || self.is_backwards_compatible_branch()
  }
}

/// Build the +app_update ... segment including any beta flags and validate toggle.
pub fn compose_app_update_arg(app_id: i64, beta: &BetaConfig, do_validate: bool) -> String {
  let mut app_update = format!("+app_update {app_id}");
  if beta.beta_in_effect() {
    app_update.push_str(&format!(" -beta {}", beta.branch));
    if beta.use_public_beta && !beta.is_backwards_compatible_branch() {
      app_update.push_str(&format!(" -betapassword {}", beta.password));
    }
  }
  if do_validate {
    app_update.push_str(" validate");
  } else {
    debug!("Skipping SteamCMD validate: VALIDATE_ON_INSTALL=0");
  }
  app_update
}

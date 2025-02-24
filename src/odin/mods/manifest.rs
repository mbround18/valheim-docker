use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
  pub name: String,
  pub dependencies: Vec<String>,
  pub description: Option<String>,
  pub version_number: String,
  pub website_url: Option<String>,
}

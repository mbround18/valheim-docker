use std::process::Command;
fn main() {
  let git_hash = Command::new("git")
    .args(["rev-parse", "HEAD"])
    .output()
    .ok()
    .filter(|output| output.status.success())
    .and_then(|output| String::from_utf8(output.stdout).ok())
    .map(|hash| hash.trim().to_string())
    .filter(|hash| !hash.is_empty())
    .unwrap_or_else(|| "unknown".to_string());
  println!("cargo:rustc-env=GIT_HASH={git_hash}");
}

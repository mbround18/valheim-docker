//! `huginn --version` must print the version and exit, not start the HTTP server.
//!
//! Release tooling smoke-tests a built binary by running `<binary> --version` (paws release
//! does exactly that). Before this flag existed huginn ignored its arguments and started
//! serving, so a smoke test would hang instead of passing.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run_with_timeout(args: &[&str]) -> (bool, String) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_huginn"))
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("failed to start huginn");

  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    if let Some(status) = child.try_wait().expect("failed to poll huginn") {
      let output = child
        .wait_with_output()
        .expect("failed to read huginn output");
      return (
        status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
      );
    }
    if Instant::now() >= deadline {
      let _ = child.kill();
      let _ = child.wait();
      panic!(
        "`huginn {}` did not exit within 10s; it started the server instead",
        args.join(" ")
      );
    }
    std::thread::sleep(Duration::from_millis(50));
  }
}

#[test]
fn version_flag_prints_version_and_exits() {
  for flag in ["--version", "-V"] {
    let (success, stdout) = run_with_timeout(&[flag]);
    assert!(success, "`huginn {flag}` should exit successfully");
    assert_eq!(
      stdout.trim(),
      format!("huginn {}", env!("CARGO_PKG_VERSION")),
      "`huginn {flag}` output"
    );
  }
}

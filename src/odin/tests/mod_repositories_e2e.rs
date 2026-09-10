//! End-to-end tests for `odin mod:install --from-var` across mod repositories.
//!
//! These run the real `odin` binary as a subprocess, the way the container's startup
//! script does, against local mock servers standing in for Thunderstore and Hexium. They
//! cover the whole path: `ts:`/`hex:` prefixes, `MODS_REPOSITORY`, wildcard resolution,
//! redirects, downloads, installation, and the state file that drives cleanup.
//!
//! The live test at the bottom talks to the real hexium.gg and is opt-in:
//!   HEXIUM_LIVE_TEST=1 cargo test -p odin --test mod_repositories_e2e -- --ignored

use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::process::{Command, Output};

/// Variables that would change what the child resolves against if inherited from the
/// developer's shell or CI environment.
const ISOLATED_VARS: [&str; 7] = [
  "MODS_REPOSITORY",
  "HEXIUM_BASE_URL",
  "HEXIUM_TOKEN",
  "THUNDERSTORE_BASE_URL",
  "THUNDERSTORE_TOKEN",
  "MODS_CONTINUE_ON_FAILURE",
  "RUST_LOG",
];

fn zip_with_manifest(name: &str) -> Vec<u8> {
  let mut buf: Vec<u8> = Vec::new();
  {
    let mut zipw = zip::ZipWriter::new(Cursor::new(&mut buf));
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    zipw.start_file("manifest.json", options).unwrap();
    zipw
      .write_all(serde_json::json!({ "name": name }).to_string().as_bytes())
      .unwrap();
    zipw
      .start_file(format!("plugins/{name}.dll"), options)
      .unwrap();
    zipw.write_all(name.as_bytes()).unwrap();
    zipw.finish().unwrap();
  }
  buf
}

/// A throwaway game directory and a way to run `odin mod:install --from-var` against it.
struct Sandbox {
  _tmp: tempfile::TempDir,
  game: PathBuf,
}

impl Sandbox {
  fn new() -> Self {
    let tmp = tempfile::tempdir().expect("tempdir");
    let game = tmp.path().join("valheim");
    std::fs::create_dir_all(&game).unwrap();
    Sandbox { _tmp: tmp, game }
  }

  fn plugin(&self, name: &str) -> PathBuf {
    self.game.join("BepInEx").join("plugins").join(name)
  }

  fn state(&self) -> serde_json::Value {
    let path = self.game.join(".staging/mods/from-var-mods.json");
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&raw).expect("state file should be JSON")
  }

  fn run(&self, mods: &str, env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_odin"));
    cmd
      .args(["mod:install", "--from-var"])
      // A clean cwd so `dotenv` cannot pick up a stray .env file.
      .current_dir(&self.game)
      .env("GAME_LOCATION", &self.game)
      .env("MODS", mods)
      .env("DOWNLOAD_STAGGER_MS", "0");
    for var in ISOLATED_VARS {
      cmd.env_remove(var);
    }
    for (key, value) in env {
      cmd.env(key, value);
    }
    cmd.output().expect("failed to run odin")
  }
}

fn logs(output: &Output) -> String {
  format!(
    "{}{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  )
}

/// Registers a Hexium package version whose download lives on the mock CDN.
fn mock_hexium_package(
  server: &mut mockito::ServerGuard,
  namespace: &str,
  name: &str,
  version: &str,
  upload_id: u32,
) -> Vec<mockito::Mock> {
  let cdn_path = format!("/upload/{upload_id}/{version}.zip");
  vec![
    server
      .mock(
        "GET",
        format!("/api/experimental/package/{namespace}/{name}/{version}/").as_str(),
      )
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(
        serde_json::json!({
          "namespace": namespace,
          "name": name,
          "version_number": version,
          "download_url": format!("{}{cdn_path}", server.url()),
        })
        .to_string(),
      )
      .create(),
    server
      .mock("GET", cdn_path.as_str())
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_with_manifest(name))
      .create(),
  ]
}

/// Registers a Thunderstore package that redirects to its CDN, as thunderstore.io does.
fn mock_thunderstore_package(
  server: &mut mockito::ServerGuard,
  namespace: &str,
  name: &str,
  version: &str,
) -> Vec<mockito::Mock> {
  let cdn_path = format!("/live/repository/packages/{namespace}-{name}-{version}.zip");
  vec![
    server
      .mock(
        "GET",
        format!("/package/download/{namespace}/{name}/{version}/").as_str(),
      )
      .with_status(302)
      .with_header("location", &cdn_path)
      .create(),
    server
      .mock("GET", cdn_path.as_str())
      .with_status(200)
      .with_header("content-type", "application/zip")
      .with_body(zip_with_manifest(name))
      .create(),
  ]
}

/// One `MODS` list mixing both repositories: a `hex:` wildcard, a `ts:` pin that overrides
/// the Hexium default, and an unprefixed entry that follows `MODS_REPOSITORY=hexium`.
/// The two Hexium mods share a version, so they also exercise distinct cache names.
#[test]
fn installs_from_both_repositories_in_one_run() {
  let mut hexium = mockito::Server::new();
  let mut thunderstore = mockito::Server::new();

  let mut mocks = vec![hexium
    .mock(
      "GET",
      "/api/experimental/frontend/c/valheim/p/Azumatt/Boxes/",
    )
    .with_status(200)
    .with_body(
      serde_json::json!({ "versions": [
        { "version_number": "1.8.18" },
        { "version_number": "1.8.17" },
      ]})
      .to_string(),
    )
    .create()];
  mocks.extend(mock_hexium_package(
    &mut hexium,
    "Azumatt",
    "Boxes",
    "1.8.18",
    48,
  ));
  mocks.extend(mock_hexium_package(
    &mut hexium,
    "Smoothbrain",
    "Building",
    "1.8.18",
    4,
  ));
  mocks.extend(mock_thunderstore_package(
    &mut thunderstore,
    "ValheimModding",
    "Jotunn",
    "2.30.0",
  ));

  let sandbox = Sandbox::new();
  let mods = "hex:Azumatt-Boxes-*\nts:ValheimModding-Jotunn-2.30.0\nSmoothbrain-Building-1.8.18";
  let output = sandbox.run(
    mods,
    &[
      ("MODS_REPOSITORY", "hexium"),
      ("HEXIUM_BASE_URL", &hexium.url()),
      ("THUNDERSTORE_BASE_URL", &thunderstore.url()),
    ],
  );
  let log = logs(&output);
  assert!(output.status.success(), "odin failed:\n{log}");

  for name in ["Boxes", "Jotunn", "Building"] {
    assert!(
      sandbox.plugin(name).join(format!("{name}.dll")).exists(),
      "{name} was not installed:\n{log}"
    );
  }
  for mock in &mocks {
    mock.assert();
  }

  // Entries are recorded verbatim, prefixes included, so the next run can reconcile them.
  let state = sandbox.state();
  let recorded: Vec<&str> = state["mods"]
    .as_array()
    .expect("mods array")
    .iter()
    .map(|m| m["url"].as_str().unwrap())
    .collect();
  assert_eq!(
    recorded,
    [
      "hex:Azumatt-Boxes-*",
      "ts:ValheimModding-Jotunn-2.30.0",
      "Smoothbrain-Building-1.8.18"
    ]
  );

  // Logs name the resolved package, not the bare version a Hexium CDN URL ends in.
  assert!(
    log.contains("Azumatt-Boxes-1.8.18"),
    "logs should name the package:\n{log}"
  );
}

/// Dropping a `hex:` entry from `MODS` uninstalls it on the next run, while the remaining
/// `ts:` mod is reused from the staging cache instead of being downloaded again.
#[test]
fn removing_a_hexium_mod_cleans_it_up_on_the_next_run() {
  let mut hexium = mockito::Server::new();
  let mut thunderstore = mockito::Server::new();
  let mut mocks = mock_hexium_package(&mut hexium, "Azumatt", "Boxes", "1.8.18", 48);
  // `create()` defaults to expecting exactly one call, which is what proves the second
  // run served Jotunn from cache.
  mocks.extend(mock_thunderstore_package(
    &mut thunderstore,
    "ValheimModding",
    "Jotunn",
    "2.30.0",
  ));
  let env = [
    ("HEXIUM_BASE_URL", hexium.url()),
    ("THUNDERSTORE_BASE_URL", thunderstore.url()),
  ];
  let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();

  let sandbox = Sandbox::new();
  let first = sandbox.run(
    "hex:Azumatt-Boxes-1.8.18\nts:ValheimModding-Jotunn-2.30.0",
    &env,
  );
  assert!(
    first.status.success(),
    "first run failed:\n{}",
    logs(&first)
  );
  assert!(sandbox.plugin("Boxes").exists());

  let second = sandbox.run("ts:ValheimModding-Jotunn-2.30.0", &env);
  assert!(
    second.status.success(),
    "second run failed:\n{}",
    logs(&second)
  );
  assert!(
    !sandbox.plugin("Boxes").exists(),
    "removed Hexium mod should be uninstalled:\n{}",
    logs(&second)
  );
  assert!(sandbox.plugin("Jotunn").join("Jotunn.dll").exists());
  for mock in &mocks {
    mock.assert();
  }
}

/// A package Hexium does not have fails the run and says where it looked.
#[test]
fn a_missing_hexium_package_fails_the_run_and_names_hexium() {
  let mut hexium = mockito::Server::new();
  let _missing = hexium
    .mock("GET", "/api/experimental/package/Nobody/Nothing/1.0.0/")
    .with_status(404)
    .with_body(r#"{"detail":"Not found."}"#)
    .create();

  let sandbox = Sandbox::new();
  let output = sandbox.run(
    "hex:Nobody-Nothing-1.0.0",
    &[("HEXIUM_BASE_URL", &hexium.url())],
  );
  let log = logs(&output);
  assert!(!output.status.success(), "run should fail:\n{log}");
  assert!(
    log.contains("Nobody-Nothing-1.0.0 was not found on Hexium"),
    "error should name the package and repository:\n{log}"
  );
}

/// Installs a real mod from hexium.gg through the binary.
#[test]
#[ignore]
fn hexium_live_install() {
  if std::env::var("HEXIUM_LIVE_TEST").unwrap_or_default() != "1" {
    eprintln!("skipping live Hexium e2e test; set HEXIUM_LIVE_TEST=1 to enable");
    return;
  }
  let sandbox = Sandbox::new();
  let output = sandbox.run("hex:Azumatt-AzuCraftyBoxes-1.8.18", &[]);
  let log = logs(&output);
  assert!(output.status.success(), "odin failed:\n{log}");
  assert!(
    sandbox
      .plugin("AzuCraftyBoxes")
      .join("AzuCraftyBoxes.dll")
      .exists(),
    "AzuCraftyBoxes.dll should be installed:\n{log}"
  );
}

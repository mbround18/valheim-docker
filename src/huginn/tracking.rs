use chrono::Utc;
use log::{error, warn};
use odin::{server::ServerInfo, utils::common_paths::saves_directory};
use redb::{Database, TableDefinition};
use serde::Serialize;
use std::path::{Path, PathBuf};

const INFO_TABLE: TableDefinition<&str, &str> = TableDefinition::new("huginn_info");
const MODS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("huginn_mods");
const LATEST_KEY: &str = "latest";

#[derive(Serialize)]
struct Snapshot<T> {
  recorded_at: i64,
  payload: T,
}

fn tracking_enabled() -> bool {
  std::env::var("HUGINN_TRACKING_BACKEND")
    .map(|v| v.eq_ignore_ascii_case("redb"))
    .unwrap_or(false)
}

fn db_path() -> PathBuf {
  PathBuf::from(format!("{}/huginn-tracking.redb", saves_directory()))
}

fn open_db(path: &Path) -> Option<Database> {
  let Some(parent) = path.parent() else {
    error!("Invalid Huginn redb path: {}", path.display());
    return None;
  };
  if let Err(e) = std::fs::create_dir_all(parent) {
    error!("Could not create tracking directory for Huginn redb: {e}");
    return None;
  }
  let db = if path.exists() {
    Database::open(path)
  } else {
    Database::create(path)
  };
  match db {
    Ok(db) => Some(db),
    Err(e) => {
      error!("Could not open Huginn redb store {}: {e}", path.display());
      None
    }
  }
}

fn write_snapshot<T: Serialize>(table_def: TableDefinition<&str, &str>, payload: T) {
  if !tracking_enabled() {
    return;
  }
  let path = db_path();
  let Some(db) = open_db(&path) else {
    return;
  };
  let serialized = match serde_json::to_string(&Snapshot {
    recorded_at: Utc::now().timestamp(),
    payload,
  }) {
    Ok(data) => data,
    Err(e) => {
      error!("Could not serialize Huginn tracking snapshot: {e}");
      return;
    }
  };
  let write_txn = match db.begin_write() {
    Ok(txn) => txn,
    Err(e) => {
      error!("Could not begin Huginn redb write transaction: {e}");
      return;
    }
  };
  {
    let mut table = match write_txn.open_table(table_def) {
      Ok(table) => table,
      Err(e) => {
        error!("Could not open Huginn redb table: {e}");
        return;
      }
    };
    let insert_result = table.insert(LATEST_KEY, serialized.as_str());
    if let Err(e) = insert_result {
      error!("Could not write Huginn redb snapshot: {e}");
      return;
    }
  }
  if let Err(e) = write_txn.commit() {
    error!("Could not commit Huginn redb write transaction: {e}");
  }
}

pub(crate) fn record_info_snapshot(info: &ServerInfo) {
  write_snapshot(INFO_TABLE, info);
}

pub(crate) fn record_mods_snapshot(mods: &[(String, Option<String>)]) {
  if mods.is_empty() {
    warn!("Skipping empty mods snapshot for Huginn redb tracking");
    return;
  }
  write_snapshot(MODS_TABLE, mods);
}

#[cfg(test)]
mod tests {
  use super::*;
  use odin::server::ServerInfo;
  use tempfile::tempdir;

  fn table_value(path: &Path, table: TableDefinition<&str, &str>) -> Option<String> {
    let db = Database::open(path).ok()?;
    let read_txn = db.begin_read().ok()?;
    let table = read_txn.open_table(table).ok()?;
    let value = table.get(LATEST_KEY).ok()??;
    Some(value.value().to_string())
  }

  #[test]
  fn redb_tracking_writes_status_and_mod_snapshots() {
    let saves = tempdir().expect("tempdir");
    std::env::set_var("SAVE_LOCATION", saves.path());
    std::env::set_var("HUGINN_TRACKING_BACKEND", "redb");

    record_info_snapshot(&ServerInfo::offline());
    record_mods_snapshot(&[(String::from("Jotunn"), Some(String::from("2.30.0")))]);

    let path = saves.path().join("huginn-tracking.redb");
    assert!(path.exists(), "expected redb tracking file to exist");
    assert!(
      table_value(&path, INFO_TABLE)
        .as_deref()
        .is_some_and(|value| value.contains("\"payload\"")),
      "expected info snapshot payload in redb table"
    );
    assert!(
      table_value(&path, MODS_TABLE)
        .as_deref()
        .is_some_and(|value| value.contains("Jotunn")),
      "expected mods snapshot payload in redb table"
    );

    std::env::remove_var("HUGINN_TRACKING_BACKEND");
    std::env::remove_var("SAVE_LOCATION");
  }
}

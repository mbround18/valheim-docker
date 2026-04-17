use crate::utils::common_paths::{game_directory, log_directory, mods_directory, saves_directory};
use serde::Serialize;
use shared::system::{
  collect_system_metrics, format_perm_summary, fs_usage_for, perm_summary_for, FsUsage,
  PermSummary, SystemMetrics,
};
use std::env;
use std::path::Path;

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;
const MIN_MEMORY_BYTES: u64 = 2 * GIB;
const WARN_AVAILABLE_BYTES: u64 = 5 * GIB;
const WARN_USED_PERCENT: f64 = 85.0;
const WARN_INODES_USED_PERCENT: f64 = 85.0;
const EXIT_SYSTEM_CHECK_FAILED: i32 = 12;

#[derive(Debug, Serialize)]
struct SystemReport {
  system: SystemMetrics,
  paths: Vec<PathObservation>,
  checks: Option<SystemChecks>,
}

#[derive(Debug, Serialize)]
struct PathObservation {
  label: &'static str,
  path: String,
  exists: bool,
  filesystem: Option<FsUsage>,
  permissions: Option<PermSummary>,
}

#[derive(Debug, Serialize)]
struct SystemChecks {
  passed: bool,
  memory: MemoryCheck,
  storage: StorageCheck,
  warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MemoryCheck {
  passed: bool,
  min_total_bytes: u64,
  total_bytes: u64,
}

#[derive(Debug, Serialize)]
struct StorageCheck {
  passed: bool,
  paths: Vec<PathCheck>,
}

#[derive(Debug, Serialize)]
struct PathCheck {
  label: &'static str,
  path: String,
  create_if_missing: bool,
  create_attempted: bool,
  create_succeeded: bool,
  exists: bool,
  is_dir: bool,
  writable: bool,
  passed: bool,
  error: Option<String>,
}

#[derive(Debug, Clone)]
struct RuntimePathSpec {
  label: &'static str,
  path: String,
  create_if_missing: bool,
}

#[derive(Debug, Clone, Default)]
struct PathPreparation {
  create_attempted: bool,
  create_succeeded: bool,
  error: Option<String>,
}

pub fn invoke(output_json: bool, run_check: bool) {
  let report = build_report(run_check);

  if output_json {
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
  } else {
    print_human_report(&report, run_check);
  }

  if run_check && report.checks.as_ref().is_some_and(|c| !c.passed) {
    std::process::exit(EXIT_SYSTEM_CHECK_FAILED);
  }
}

fn build_report(include_checks: bool) -> SystemReport {
  let system = collect_system_metrics();
  let path_specs = runtime_paths();
  let preparations = if include_checks {
    prepare_runtime_paths(&path_specs)
  } else {
    vec![PathPreparation::default(); path_specs.len()]
  };
  let paths = path_specs
    .iter()
    .map(|spec| PathObservation {
      label: spec.label,
      exists: Path::new(&spec.path).exists(),
      filesystem: fs_usage_for(&spec.path),
      permissions: perm_summary_for(&spec.path),
      path: spec.path.clone(),
    })
    .collect::<Vec<_>>();

  let checks = if include_checks {
    Some(run_checks(&system, &path_specs, &paths, &preparations))
  } else {
    None
  };

  SystemReport {
    system,
    paths,
    checks,
  }
}

fn run_checks(
  system: &SystemMetrics,
  specs: &[RuntimePathSpec],
  paths: &[PathObservation],
  preparations: &[PathPreparation],
) -> SystemChecks {
  let memory = MemoryCheck {
    passed: system.total_memory_bytes >= MIN_MEMORY_BYTES,
    min_total_bytes: MIN_MEMORY_BYTES,
    total_bytes: system.total_memory_bytes,
  };

  let path_checks = paths
    .iter()
    .zip(specs.iter())
    .zip(preparations.iter())
    .map(|((p, spec), prep)| {
      let is_dir = p.permissions.as_ref().is_some_and(|perm| perm.is_dir);
      let writable = p.permissions.as_ref().is_some_and(|perm| perm.can_write);
      let passed = prep.error.is_none() && p.exists && is_dir && writable;
      PathCheck {
        label: spec.label,
        path: p.path.clone(),
        create_if_missing: spec.create_if_missing,
        create_attempted: prep.create_attempted,
        create_succeeded: prep.create_succeeded,
        exists: p.exists,
        is_dir,
        writable,
        passed,
        error: prep.error.clone(),
      }
    })
    .collect::<Vec<_>>();

  let warnings = paths
    .iter()
    .filter_map(storage_warning_for)
    .collect::<Vec<_>>();

  let storage = StorageCheck {
    passed: path_checks.iter().all(|p| p.passed),
    paths: path_checks,
  };

  let passed = memory.passed && storage.passed;

  SystemChecks {
    passed,
    memory,
    storage,
    warnings,
  }
}

fn storage_warning_for(path: &PathObservation) -> Option<String> {
  let fs = path.filesystem.as_ref()?;

  if fs.available_bytes < WARN_AVAILABLE_BYTES {
    return Some(format!(
      "Low available space on {}: {} available",
      fs.path,
      human_bytes(fs.available_bytes)
    ));
  }

  if fs.used_percent >= WARN_USED_PERCENT {
    return Some(format!(
      "High disk usage on {}: {:.1}% used",
      fs.path, fs.used_percent
    ));
  }

  if fs.inodes_total > 0 && fs.inodes_used_percent >= WARN_INODES_USED_PERCENT {
    return Some(format!(
      "High inode usage on {}: {:.1}% used",
      fs.path, fs.inodes_used_percent
    ));
  }

  None
}

fn runtime_paths() -> Vec<RuntimePathSpec> {
  let home = env::var("HOME").unwrap_or_else(|_| String::from("/home/steam"));
  let game = game_directory();
  let saves = saves_directory();
  let mods = mods_directory();
  let logs = env::var("LOG_LOCATION").unwrap_or_else(|_| log_directory());
  let backups = env::var("BACKUP_LOCATION").unwrap_or_else(|_| format!("{}/backups", game));

  let candidates = vec![
    RuntimePathSpec {
      label: "home",
      path: home,
      create_if_missing: false,
    },
    RuntimePathSpec {
      label: "game",
      path: game.clone(),
      create_if_missing: true,
    },
    RuntimePathSpec {
      label: "saves",
      path: saves,
      create_if_missing: true,
    },
    RuntimePathSpec {
      label: "mods",
      path: mods,
      create_if_missing: true,
    },
    RuntimePathSpec {
      label: "backups",
      path: backups,
      create_if_missing: true,
    },
    RuntimePathSpec {
      label: "logs",
      path: logs,
      create_if_missing: true,
    },
    RuntimePathSpec {
      label: "tmp",
      path: String::from("/tmp"),
      create_if_missing: false,
    },
  ];

  let mut paths = Vec::new();
  for spec in candidates {
    if !paths
      .iter()
      .any(|existing: &RuntimePathSpec| existing.path == spec.path)
    {
      paths.push(spec);
    }
  }
  paths
}

fn prepare_runtime_paths(specs: &[RuntimePathSpec]) -> Vec<PathPreparation> {
  specs.iter().map(prepare_runtime_path).collect()
}

fn prepare_runtime_path(spec: &RuntimePathSpec) -> PathPreparation {
  let path = Path::new(&spec.path);

  if path.exists() {
    if !path.is_dir() {
      return PathPreparation {
        error: Some(String::from("Path exists but is not a directory")),
        ..PathPreparation::default()
      };
    }
    return PathPreparation::default();
  }

  if !spec.create_if_missing {
    return PathPreparation {
      error: Some(String::from("Path does not exist")),
      ..PathPreparation::default()
    };
  }

  let mut prep = PathPreparation {
    create_attempted: true,
    ..PathPreparation::default()
  };

  match std::fs::create_dir_all(path) {
    Ok(_) => {
      prep.create_succeeded = true;
    }
    Err(e) => {
      prep.error = Some(format!("Failed to create directory recursively: {}", e));
    }
  }

  prep
}

fn print_human_report(report: &SystemReport, include_checks: bool) {
  let s = &report.system;
  let used_disk = s.total_disk_bytes.saturating_sub(s.available_disk_bytes);
  let disk_used_pct = if s.total_disk_bytes > 0 {
    used_disk as f64 / s.total_disk_bytes as f64 * 100.0
  } else {
    0.0
  };
  let mem_used_pct = percent(s.used_memory_bytes, s.total_memory_bytes);
  let swap_used_pct = percent(s.used_swap_bytes, s.total_swap_bytes);

  println!("System:");
  println!(
    "  Memory: {} used / {} total ({:.1}%)",
    human_bytes(s.used_memory_bytes),
    human_bytes(s.total_memory_bytes),
    mem_used_pct
  );
  println!(
    "  Swap:   {} used / {} total ({:.1}%)",
    human_bytes(s.used_swap_bytes),
    human_bytes(s.total_swap_bytes),
    swap_used_pct
  );
  println!("  CPUs:   {} logical", s.cpu_num_logical);
  println!(
    "  Load:   {:.2} {:.2} {:.2}",
    s.load_average_one, s.load_average_five, s.load_average_fifteen
  );
  println!(
    "  Disk:   {} used / {} total ({:.1}%), {} available (aggregate)",
    human_bytes(used_disk),
    human_bytes(s.total_disk_bytes),
    disk_used_pct,
    human_bytes(s.available_disk_bytes)
  );

  println!("Paths:");
  for p in &report.paths {
    println!("  {} ({})", p.path, p.label);

    match &p.filesystem {
      Some(fs) => println!(
        "    FS:    used={} ({:.1}%), avail={}, total={}, inodes={:.1}% used",
        human_bytes(fs.used_bytes),
        fs.used_percent,
        human_bytes(fs.available_bytes),
        human_bytes(fs.total_bytes),
        fs.inodes_used_percent
      ),
      None => println!("    FS:    unavailable"),
    }

    match &p.permissions {
      Some(perm) => println!("    {}", format_perm_summary(perm)),
      None => println!("    Perms: unavailable"),
    }
  }

  if include_checks {
    if let Some(checks) = &report.checks {
      println!("Checks:");
      println!(
        "  Memory >= {}: {} (detected {})",
        human_bytes(checks.memory.min_total_bytes),
        if checks.memory.passed { "PASS" } else { "FAIL" },
        human_bytes(checks.memory.total_bytes)
      );
      println!(
        "  Storage paths writable: {}",
        if checks.storage.passed {
          "PASS"
        } else {
          "FAIL"
        }
      );
      for p in &checks.storage.paths {
        println!(
          "    {} ({}) -> exists={}, dir={}, writable={}, create_if_missing={}, create_attempted={}, create_succeeded={}, result={}",
          p.path,
          p.label,
          yes_no(p.exists),
          yes_no(p.is_dir),
          yes_no(p.writable),
          yes_no(p.create_if_missing),
          yes_no(p.create_attempted),
          yes_no(p.create_succeeded),
          if p.passed { "PASS" } else { "FAIL" }
        );
        if let Some(error) = &p.error {
          println!("      error: {}", error);
        }
      }

      if checks.warnings.is_empty() {
        println!("  Warnings: none");
      } else {
        println!("  Warnings:");
        for w in &checks.warnings {
          println!("    - {}", w);
        }
      }

      if checks.storage.paths.iter().any(|p| !p.passed) {
        println!("  Remediation:");
        println!("    This container runs rootless and cannot repair host bind mount ownership.");
        println!("    Ensure the host directories mapped to failed paths are writable by the container user.");
        println!("    Example: sudo chown -R <uid>:<gid> <host-path>");
        println!("    Example: sudo chmod -R u+rwX,g+rwX <host-path>");
        println!("    Also ensure Compose uses a matching user: `user: \"UID:GID\"`.");
      }

      println!("  Overall: {}", if checks.passed { "PASS" } else { "FAIL" });
    }
  }
}

fn percent(used: u64, total: u64) -> f64 {
  if total == 0 {
    0.0
  } else {
    used as f64 / total as f64 * 100.0
  }
}

fn human_bytes(bytes: u64) -> String {
  if bytes >= GIB {
    format!("{:.1} GiB", bytes as f64 / GIB as f64)
  } else if bytes >= MIB {
    format!("{:.1} MiB", bytes as f64 / MIB as f64)
  } else if bytes >= 1024 {
    format!("{:.1} KiB", bytes as f64 / 1024.0)
  } else {
    format!("{} B", bytes)
  }
}

fn yes_no(value: bool) -> &'static str {
  if value {
    "yes"
  } else {
    "no"
  }
}

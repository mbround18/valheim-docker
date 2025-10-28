use nix::sys::statvfs::statvfs;
use nix::unistd::{Gid, Uid, getgroups};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use sysinfo::{Disks, System};

#[derive(Debug, Clone)]
pub struct SystemMetrics {
  pub total_memory_bytes: u64,
  pub used_memory_bytes: u64,
  pub total_swap_bytes: u64,
  pub used_swap_bytes: u64,
  pub total_disk_bytes: u64,
  pub available_disk_bytes: u64,
  pub cpu_num_logical: usize,
  pub load_average_one: f64,
  pub load_average_five: f64,
  pub load_average_fifteen: f64,
}

pub fn collect_system_metrics() -> SystemMetrics {
  let mut sys = System::new();
  sys.refresh_memory();
  sys.refresh_cpu_all();

  // Disk aggregation
  let disks = Disks::new_with_refreshed_list();
  let mut total_disk_bytes: u64 = 0;
  let mut available_disk_bytes: u64 = 0;
  for d in disks.list() {
    total_disk_bytes += d.total_space();
    available_disk_bytes += d.available_space();
  }

  let mem_total = sys.total_memory();
  let mem_used = sys.used_memory();
  let swap_total = sys.total_swap();
  let swap_used = sys.used_swap();
  let cpus = sys.cpus().len();
  let load = sysinfo::System::load_average();

  SystemMetrics {
    total_memory_bytes: mem_total * 1024, // sysinfo reports in KiB
    used_memory_bytes: mem_used * 1024,
    total_swap_bytes: swap_total * 1024,
    used_swap_bytes: swap_used * 1024,
    total_disk_bytes,
    available_disk_bytes,
    cpu_num_logical: cpus,
    load_average_one: load.one,
    load_average_five: load.five,
    load_average_fifteen: load.fifteen,
  }
}

#[derive(Debug, Clone)]
pub struct FsUsage {
  pub path: String,
  pub total_bytes: u64,
  pub available_bytes: u64,
  pub used_bytes: u64,
  pub used_percent: f64,
  pub inodes_total: u64,
  pub inodes_free: u64,
  pub inodes_used: u64,
  pub inodes_used_percent: f64,
}

pub fn fs_usage_for<P: AsRef<Path>>(path: P) -> Option<FsUsage> {
  let p = path.as_ref();
  let s = statvfs(p).ok()?;
  let total_bytes = (s.blocks() * s.block_size()) as u64;
  let available_bytes = (s.blocks_available() * s.block_size()) as u64;
  let used_bytes = total_bytes.saturating_sub(available_bytes);
  let used_percent = if total_bytes > 0 {
    (used_bytes as f64) / (total_bytes as f64) * 100.0
  } else {
    0.0
  };

  let inodes_total = s.files() as u64;
  let inodes_free = s.files_free() as u64;
  let inodes_used = inodes_total.saturating_sub(inodes_free);
  let inodes_used_percent = if inodes_total > 0 {
    (inodes_used as f64) / (inodes_total as f64) * 100.0
  } else {
    0.0
  };

  Some(FsUsage {
    path: p.display().to_string(),
    total_bytes,
    available_bytes,
    used_bytes,
    used_percent,
    inodes_total,
    inodes_free,
    inodes_used,
    inodes_used_percent,
  })
}

#[derive(Debug, Clone)]
pub struct PermSummary {
  pub path: String,
  pub owner_uid: u32,
  pub owner_gid: u32,
  pub mode_octal: u32,
  pub is_dir: bool,
  pub access_class: &'static str, // "owner" | "group" | "other"
  pub can_read: bool,
  pub can_write: bool,
}

fn mode_string(mode: u32, is_dir: bool) -> String {
  let mut s = String::new();
  s.push(if is_dir { 'd' } else { '-' });
  let triples = [
    (0o400, 0o200, 0o100),
    (0o040, 0o020, 0o010),
    (0o004, 0o002, 0o001),
  ];
  for (r, w, x) in triples {
    s.push(if mode & r != 0 { 'r' } else { '-' });
    s.push(if mode & w != 0 { 'w' } else { '-' });
    s.push(if mode & x != 0 { 'x' } else { '-' });
  }
  s
}

pub fn perm_summary_for<P: AsRef<Path>>(path: P) -> Option<PermSummary> {
  let p = path.as_ref();
  let md = std::fs::metadata(p).ok()?;
  let mode = md.mode() & 0o7777; // capture perm bits, ignore file type when computing octal display
  let is_dir = md.is_dir();
  let file_uid = md.uid();
  let file_gid = md.gid();

  let euid = Uid::effective().as_raw();
  let egid = Gid::effective().as_raw();
  let groups: Vec<u32> = getgroups()
    .map(|v| v.into_iter().map(|g| g.as_raw()).collect())
    .unwrap_or_default();

  // Determine which class applies: owner, group (primary or supplemental), or other
  let (class, r, w) = if euid == file_uid {
    ("owner", (mode & 0o400) != 0, (mode & 0o200) != 0)
  } else if egid == file_gid || groups.contains(&file_gid) {
    ("group", (mode & 0o040) != 0, (mode & 0o020) != 0)
  } else {
    ("other", (mode & 0o004) != 0, (mode & 0o002) != 0)
  };

  Some(PermSummary {
    path: p.display().to_string(),
    owner_uid: file_uid,
    owner_gid: file_gid,
    mode_octal: mode & 0o777,
    is_dir,
    access_class: class,
    can_read: r,
    can_write: w,
  })
}

pub fn format_perm_summary(perm: &PermSummary) -> String {
  let mode_str = mode_string(perm.mode_octal, perm.is_dir);
  format!(
    "Perms {}: {} (0o{:03o}), owner={}:{}, class={}, read={}, write={}",
    perm.path,
    mode_str,
    perm.mode_octal,
    perm.owner_uid,
    perm.owner_gid,
    perm.access_class,
    if perm.can_read { "yes" } else { "no" },
    if perm.can_write { "yes" } else { "no" },
  )
}

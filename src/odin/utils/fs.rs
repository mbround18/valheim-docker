use std::fs;
use std::path::Path;

/// Remove a path cautiously: supports files, dirs, and symlinks without following them.
pub fn remove_path_cautious(p: &Path) -> std::io::Result<()> {
  let meta = fs::symlink_metadata(p)?;
  let ft = meta.file_type();
  if ft.is_symlink() {
    fs::remove_file(p)
  } else if ft.is_dir() {
    fs::remove_dir_all(p)
  } else {
    fs::remove_file(p)
  }
}

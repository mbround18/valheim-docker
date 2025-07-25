use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

/// Normalizes paths by replacing backslashes with forward slashes.
fn normalize_path(path: &Path) -> PathBuf {
  let path_str = path.to_string_lossy().replace('\\', "/");
  PathBuf::from(path_str)
}

/// Moves and normalizes contents from the source directory to a temporary directory,
/// then moves them back to the original directory with normalized paths.
pub fn normalize_paths(src_dir: &Path) -> std::io::Result<()> {
  // Ensure the source directory exists
  if !src_dir.is_dir() {
    return Err(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      format!("Source directory {src_dir:?} does not exist"),
    ));
  }

  // Create a temporary directory
  let temp_dir = tempdir()?;
  let temp_root = temp_dir.path();

  // Move and normalize contents to the temporary directory
  for entry in WalkDir::new(src_dir).into_iter().filter_map(Result::ok) {
    let src_path = entry.path();
    // Compute the path relative to src_dir
    let relative_path = src_path
      .strip_prefix(src_dir)
      .map_err(std::io::Error::other)?;
    let normalized_relative_path = normalize_path(relative_path);
    let temp_dest_path = temp_root.join(&normalized_relative_path);

    if src_path.is_dir() {
      fs::create_dir_all(&temp_dest_path)?;
    } else {
      if let Some(parent) = temp_dest_path.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::rename(src_path, &temp_dest_path)?;
    }
  }

  // Remove the original source directory and recreate it empty
  fs::remove_dir_all(src_dir)?;
  fs::create_dir_all(src_dir)?;

  // Move normalized contents back to the original directory
  for entry in WalkDir::new(temp_root).into_iter().filter_map(Result::ok) {
    let entry_path = entry.path();
    // Compute path relative to the temp_root
    let relative_path = entry_path
      .strip_prefix(temp_root)
      .map_err(std::io::Error::other)?;
    let original_dest_path = src_dir.join(relative_path);

    if entry_path.is_dir() {
      fs::create_dir_all(&original_dest_path)?;
    } else {
      if let Some(parent) = original_dest_path.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::rename(entry_path, &original_dest_path)?;
    }
  }

  // temp_dir is automatically deleted when it goes out of scope.
  Ok(())
}

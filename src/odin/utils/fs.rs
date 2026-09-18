use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

/// Lowercase hex SHA-256 of a file, read in chunks so large mod archives stay out of memory.
pub fn sha256_file_hex(path: &Path) -> std::io::Result<String> {
  let mut file = fs::File::open(path)?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 8192];
  loop {
    let n = file.read(&mut buf)?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  Ok(hex::encode(hasher.finalize()))
}

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sha256_file_hex_matches_known_digest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("abc.txt");
    fs::write(&path, b"abc").unwrap();
    assert_eq!(
      sha256_file_hex(&path).unwrap(),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(sha256_file_hex(&dir.path().join("missing")).is_err());
  }
}

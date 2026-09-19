pub mod config;
pub mod discord;

// use crate::executable::create_execution;
use crate::utils::get_working_dir;
use log::{debug, error, info};
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::process::exit;

pub fn create_file(path: &str) -> File {
  let output_path = Path::new(path);
  if let Ok(file) = File::create(output_path) {
    file
  } else {
    error!("Failed to create {path}");
    exit(1)
  }
}

pub trait FileManager {
  fn path(&self) -> String;
  fn exists(&self) -> bool {
    Path::new(self.path().as_str()).exists()
  }
  fn read(&self) -> String {
    if self.exists() {
      fs::read_to_string(self.path()).unwrap()
    } else {
      String::new()
    }
  }
  fn write(&self, content: String) -> bool {
    debug!("Writing file path: {}", self.path().as_str());
    create_dir_all(Path::new(self.path().as_str()).parent().unwrap()).unwrap();
    let mut file = create_file(self.path().as_str());
    if let Ok(()) = file.write_all(content.as_bytes()) {
      info!("Successfully written {}", self.path());
      true
    } else {
      error!("Failed to write {}", self.path());
      false
    }
  }
}

pub struct ManagedFile {
  pub(crate) name: String,
}

impl FileManager for ManagedFile {
  fn path(&self) -> String {
    let supplied_path = Path::new(self.name.as_str());
    debug!("Managed File: Path - {}", self.name.as_str());
    if supplied_path.parent().unwrap().exists() {
      supplied_path.to_str().unwrap().to_string()
    } else {
      format!("{}/{}", get_working_dir(), self.name)
    }
  }
}

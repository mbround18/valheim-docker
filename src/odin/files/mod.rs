pub mod config;
pub mod discord;

use crate::executable::create_execution;
use crate::utils::get_working_dir;
use log::{debug, error, info};
use std::fs;
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use std::path::Path;
use std::process::exit;

pub fn create_file(path: &str) -> File {
  let output_path = Path::new(path);
  match File::create(output_path) {
    Ok(file) => file,
    Err(_) => {
      error!("Failed to create {}", path);
      exit(1)
    }
  }
}

pub trait FileManager {
  fn path(&self) -> String;
  fn exists(&self) -> bool {
    Path::new(self.path().as_str()).exists()
  }
  fn remove(&self) -> bool {
    match remove_file(self.path()) {
      Ok(_) => {
        info!("Successfully deleted {}", self.path());
        true
      }
      Err(_) => {
        error!("Did not find or could not delete {}", self.path());
        false
      }
    }
  }
  fn read(&self) -> String {
    if self.exists() {
      fs::read_to_string(self.path()).unwrap()
    } else {
      "".to_string()
    }
  }
  fn write(&self, content: String) -> bool {
    debug!("Writing file path: {}", self.path().as_str());
    create_dir_all(Path::new(self.path().as_str()).parent().unwrap()).unwrap();
    let mut file = create_file(self.path().as_str());
    match file.write_all(content.as_bytes()) {
      Ok(_) => {
        info!("Successfully written {}", self.path());
        true
      }
      _ => {
        error!("Failed to write {}", self.path());
        false
      }
    }
  }
  fn set_executable(&self) -> bool {
    if let Ok(_output) = create_execution("chmod")
      .args(&["+x", self.path().as_str()])
      .output()
    {
      info!("Successfully set {} to executable", self.path());
      true
    } else {
      error!("Unable to set {} to executable", self.path());
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

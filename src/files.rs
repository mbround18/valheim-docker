use log::{error, info};

use std::fs;
use std::fs::{remove_file, File};
use std::io::Write;
use std::path::Path;
use std::process::exit;

use crate::executable::create_execution;
use crate::utils::{environment::fetch_var, get_working_dir, VALHEIM_EXECUTABLE_NAME};

pub struct ValheimArguments {
  pub port: String,
  pub name: String,
  pub world: String,
  pub public: String,
  pub password: String,
  pub command: String,
}

impl ValheimArguments {
  pub fn from_env() -> Self {
    let server_executable = &[get_working_dir(), VALHEIM_EXECUTABLE_NAME.to_string()].join("/");
    let command = match fs::canonicalize(server_executable) {
      std::result::Result::Ok(command_path) => command_path.to_str().unwrap().to_string(),
      std::result::Result::Err(_) => {
        error!("Failed to find server executable! Please run `odin install`");
        exit(1)
      }
    };

    Self {
      port: fetch_var("PORT", "2456"),
      name: fetch_var("NAME", "Valheim Docker"),
      world: fetch_var("WORLD", "Dedicated"),
      public: fetch_var("PUBLIC", "1"),
      password: fetch_var("PASSWORD", "12345"),
      command,
    }
  }
}

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
    if supplied_path.exists() {
      supplied_path.to_str().unwrap().to_string()
    } else {
      format!("{}/{}", get_working_dir(), self.name)
    }
  }
}

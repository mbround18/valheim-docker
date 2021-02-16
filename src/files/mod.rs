pub mod config;
use crate::executable::create_execution;
use crate::utils::get_working_dir;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{remove_file, File};
use std::io::Write;
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct ValheimArguments {
    pub(crate) port: String,
    pub(crate) name: String,
    pub(crate) world: String,
    pub(crate) public: String,
    pub(crate) password: String,
    pub(crate) command: String,
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
        match File::create(self.path()) {
            Ok(mut file) => match file.write_all(content.as_bytes()) {
                Ok(_) => {
                    info!("Successfully written {}", self.path());
                    true
                }
                _ => {
                    error!("Failed to write {}", self.path());
                    false
                }
            },
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

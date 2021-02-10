pub mod start_server_rusty;

use crate::utils::get_working_dir;
use std::path::Path;
use std::fs::{remove_file, File};
use crate::executable::create_execution;
use std::io::Write;
use log::{info,error};
use std::fs;

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
            },
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
            Ok(mut file) => {
                match file.write_all(content.as_bytes()) {
                    Ok(_) => {
                        info!("Successfully written {}", self.path());
                        true
                    },
                    _ => {
                        error!("Failed to write {}", self.path());
                        false
                    }
                }
            }
            _ => {
                error!("Failed to write {}", self.path());
                false
            }
        }
    }
    fn set_executable(&self) -> bool {
        if let Ok(_output) = create_execution("chmod").args(&["+x", self.path().as_str()]).output() {
            info!("Successfully set {} to executable", self.path());
            true
        }
        else {
            error!("Unable to set {} to executable", self.path());
            false
        }
    }
}

pub struct ManagedFile {
    pub(crate) name: &'static str
}

impl FileManager for ManagedFile {
    fn path(&self) -> String{
        format!("{}/{}", get_working_dir(), self.name)
    }
}

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::{fs, io};

use crate::files::config::{config_file, write_config};
use crate::files::discord::{discord_file, write_discord};

/// See: https://user-images.githubusercontent.com/34519392/273088066-b9c94664-9eef-419d-999a-8b8798462dee.PNG
/// for a list of modifiers
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Modifiers {
  /// The name of the modifier
  pub name: String,

  /// The value of the modifier
  pub value: String,
}

impl From<String> for Modifiers {
  /// Creates a new modifier from a string
  fn from(value: String) -> Self {
    let mut split = value.split('=');
    let name = split.next().unwrap().to_string();
    let value = split.next().unwrap().to_string();
    Modifiers { name, value }
  }
}

pub struct Configuration {
  /// Sets the name of the server, (Can be set with ENV variable NAME)
  pub name: String,

  /// Sets the servers executable path.
  pub server_executable: String,

  /// Sets the port of the server, (Can be set with ENV variable PORT)
  pub port: u16,

  /// Sets the world of the server, (Can be set with ENV variable WORLD)
  pub world: String,

  /// Sets the password of the server, (Can be set with ENV variable PASSWORD)
  pub password: String,

  /// Sets the public state of the server, (Can be set with ENV variable PUBLIC)
  pub public: bool,

  /// Sets flag preset for launching the server, (Can be set with ENV variable PRESET)
  pub preset: Option<String>,

  /// Sets flag modifiers for launching the server, (Can be set with ENV variable MODIFIERS)
  pub modifiers: Option<Vec<Modifiers>>,

  /// Sets flag set_key for launching the server, (Can be set with ENV variable SET_KEY)
  pub set_key: Option<String>,

  /// Sets the save interval in seconds
  pub save_interval: Option<u16>,
}

impl Configuration {
  /// Creates a new configuration
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    name: String,
    server_executable: String,
    port: u16,
    world: String,
    password: String,
    public: bool,
    preset: Option<String>,
    modifiers: Option<Vec<Modifiers>>,
    set_key: Option<String>,
    save_interval: Option<u16>,
  ) -> Self {
    Configuration {
      name,
      server_executable,
      port,
      world,
      password,
      public,
      preset,
      modifiers,
      set_key,
      save_interval,
    }
  }

  fn check_permissions(&self, path: &Path) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();

    if metadata.is_dir() {
      info!("Checking directory permissions: {:?}", path);
      if permissions.mode() & 0o700 == 0o700 {
        Ok(())
      } else {
        Err(io::Error::new(
          io::ErrorKind::PermissionDenied,
          "Directory does not have adequate permissions",
        ))
      }
    } else {
      info!("Checking file permissions: {:?}", path);
      if permissions.mode() & 0o600 == 0o600 {
        Ok(())
      } else {
        Err(io::Error::new(
          io::ErrorKind::PermissionDenied,
          "File does not have adequate permissions",
        ))
      }
    }
  }

  fn perform_preflight_checks(&self) -> Result<(), Box<dyn std::error::Error>> {
    let paths_to_check = vec![
      Path::new("/home/steam/valheim"),
      Path::new("/home/steam/scripts"),
      Path::new("/home/steam/.bashrc"),
    ];

    for path in paths_to_check {
      if !path.exists() {
        return Err(Box::new(io::Error::new(
          io::ErrorKind::NotFound,
          format!("Path does not exist: {:?}", path),
        )));
      }

      if let Err(e) = self.check_permissions(path) {
        return Err(Box::new(e));
      }
    }

    Ok(())
  }

  /// Invokes the configuration by writing the config file
  pub async fn invoke(self) -> Result<(), Box<dyn std::error::Error>> {
    self.perform_preflight_checks()?;

    debug!("Pulling config file...");
    let config = config_file();
    debug!("Writing config file...");
    write_config(config, self);
    debug!("Pulling Discord config file...");
    let discord = discord_file();
    debug!("Writing Discord config file...");
    write_discord(&discord);

    Ok(())
  }
}

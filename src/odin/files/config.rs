use std::{fs, path::PathBuf, process::exit};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use crate::commands::configure::{Configuration, Modifiers};
use crate::files::{FileManager, ManagedFile};
use crate::traits::AsOneOrZero;
use crate::utils::environment::fetch_var;

const ODIN_CONFIG_FILE_VAR: &str = "ODIN_CONFIG_FILE";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ValheimArguments {
  /// The port of the server, (Can be set with ENV variable PORT)
  pub(crate) port: String,

  /// The name of the server, (Can be set with ENV variable NAME)
  pub(crate) name: String,

  /// The world of the server, (Can be set with ENV variable WORLD)
  pub(crate) world: String,

  /// The public state of the server, (Can be set with ENV variable PUBLIC)
  pub(crate) public: String,

  /// The password of the server, (Can be set with ENV variable PASSWORD)
  pub(crate) password: String,

  /// The command to launch the server
  pub(crate) command: String,

  /// The preset for launching the server, (Can be set with ENV variable PRESET)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) preset: Option<String>,

  /// The modifiers for launching the server, (Can be set with ENV variable MODIFIERS)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) modifiers: Option<Vec<Modifiers>>,

  /// The set_key for launching the server, (Can be set with ENV variable SET_KEY)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) set_key: Option<String>,

  /// Sets the save interval in seconds
  #[serde(skip_serializing_if = "Option::is_none")]
  pub save_interval: Option<u16>,
}

impl From<Configuration> for ValheimArguments {
  /// Creates a new ValheimArguments from a Configuration
  fn from(value: Configuration) -> Self {
    let command = match fs::canonicalize(PathBuf::from(value.server_executable)) {
      Ok(command_path) => command_path.to_str().unwrap().to_string(),
      Err(_) => {
        error!("Failed to find server executable! Please run `odin install`");
        exit(1)
      }
    };

    ValheimArguments {
      port: value.port.to_string(),
      name: value.name,
      world: value.world,
      public: value.public.as_string(),
      password: value.password,
      command,
      preset: value.preset,
      modifiers: value.modifiers,
      set_key: value.set_key,
      save_interval: value.save_interval,
    }
  }
}

impl TryInto<Vec<String>> for ValheimArguments {
  type Error = String;

  /// Converts the ValheimArguments into a vector of strings
  fn try_into(self) -> Result<Vec<String>, Self::Error> {
    let mut args = Vec::new();
    // Sets the port of the server, (Can be set with ENV variable PORT)
    let port = fetch_var("PORT", &self.port);
    debug!("Setting port to: {}", &port);
    args.push(String::from("-port"));
    args.push(port);

    // Sets the name of the server, (Can be set with ENV variable NAME)
    let name = fetch_var("NAME", &self.name);
    debug!("Setting name to: {}", &name);
    args.push(String::from("-name"));
    // Arg processor needs the quotes around the name if it has spaces
    args.push(format!("'{}'", name));

    // Sets the world of the server, (Can be set with ENV variable WORLD)
    let world = fetch_var("WORLD", &self.world);
    debug!("Setting world to: {}", &world);
    args.push(String::from("-world"));
    args.push(world);

    // Determines if the server is public or not
    let public = fetch_var("PUBLIC", &self.public);
    debug!("Setting public to: {}", &public);
    args.push(String::from("-public"));
    args.push(public.clone());

    // Sets the save interval in seconds
    if let Some(save_interval) = &self.save_interval {
      let interval = save_interval.to_string();
      debug!("Setting save interval to: {}", &interval);
      args.push(String::from("-saveinterval"));
      args.push(interval);
    };

    // Add set_key to the command - supports multiple keys separated by commas
    if let Some(set_key) = &self.set_key {
      set_key
        .split(',')
        .map(|key| key.trim())
        .filter(|key| !key.is_empty())
        .for_each(|key| {
          debug!("Setting set_key to: {}", &key);
          args.push(String::from("-setkey"));
          args.push(key.to_string());
        });
    };

    // Add preset to the command
    if let Some(preset) = &self.preset {
      debug!("Setting preset to: {}", &preset);
      args.push(String::from("-preset"));
      args.push(preset.to_string());
    };

    // Add modifiers to the command
    if let Some(modifiers) = &self.modifiers {
      modifiers.iter().for_each(|modifier| {
        debug!(
          "Setting modifier to: {} {}",
          &modifier.name, &modifier.value
        );
        args.push(String::from("-modifier"));
        args.push(modifier.name.to_string());
        args.push(modifier.value.to_string());
      });
    };

    // Handle password logic similar to configure_server_options
    let is_public = self.public.eq("1");
    let is_vanilla = fetch_var("TYPE", "vanilla").eq_ignore_ascii_case("vanilla");
    let no_password = self.password.is_empty();

    if !is_public && !is_vanilla && no_password {
      debug!("No password found, skipping password flag.");
    } else if no_password && (is_public || is_vanilla) {
      return Err(String::from("Cannot run your server with no password! PUBLIC must be 0 and cannot be a Vanilla type server."));
    } else {
      debug!("Password found, adding password flag.");
      args.push(String::from("-password"));
      // Arg processor needs the quotes around the password if it has spaces
      args.push(format!("'{}'", self.password));
    }

    // Enable crossplay if the environment variable is set to 1
    if fetch_var("ENABLE_CROSSPLAY", "0").eq("1") {
      args.push(String::from("-crossplay"));
      debug!("Crossplay enabled");
    }

    // Add base Unity arguments
    args.push(String::from("-nographics"));
    args.push(String::from("-batchmode"));

    // Add extra arguments from SERVER_EXTRA_LAUNCH_ARGS
    let extra_launch_args = fetch_var("SERVER_EXTRA_LAUNCH_ARGS", "");
    if !extra_launch_args.is_empty() {
      for arg in extra_launch_args.split(' ') {
        if !arg.is_empty() {
          args.push(String::from(arg));
        }
      }
    }

    // Add extra arguments from the environment variable ADDITIONAL_SERVER_ARGS
    if let Ok(extra_args) = std::env::var("ADDITIONAL_SERVER_ARGS") {
      let additional_args = String::from(extra_args.trim_start_matches('"').trim_end_matches('"'));
      debug!("Adding additional arguments! {additional_args}");
      args.push(additional_args);
    }

    // Tack on save dir at the end
    use crate::utils::common_paths::saves_directory;
    args.push(String::from("-savedir"));
    args.push(saves_directory());

    Ok(args)
  }
}

/// Loads the configuration from the config file
pub fn load_config() -> ValheimArguments {
  let file = config_file();
  let config = read_config(file);

  debug!("Checking password compliance...");
  if config.password.len() < 5 && !config.password.is_empty() {
    error!("The supplied password is too short! It must be 5 characters or greater!");
    exit(1);
  }
  config
}

/// Creates a new config file
pub fn config_file() -> ManagedFile {
  let name = fetch_var(ODIN_CONFIG_FILE_VAR, "config.json");
  debug!("Config file set to: {name}");
  ManagedFile { name }
}

/// Reads the config file
pub fn read_config(config: ManagedFile) -> ValheimArguments {
  let content = config.read();
  if content.is_empty() {
    panic!("Please initialize odin with `odin configure`. See `odin configure --help`")
  }
  serde_json::from_str(content.as_str()).unwrap()
}

/// Writes the config file
pub fn write_config(config: ManagedFile, args: Configuration) -> bool {
  let content = ValheimArguments::from(args);

  let content_to_write = serde_json::to_string_pretty(&content).unwrap();
  debug!(
    "Writing config content: \n{}",
    serde_json::to_string_pretty(&content).unwrap()
  );
  config.write(content_to_write)
}

#[cfg(test)]
mod tests {
  use std::env;
  use std::env::current_dir;

  use rand::Rng;

  use super::*;

  #[test]
  #[should_panic(
    expected = "Please initialize odin with `odin configure`. See `odin configure --help`"
  )]
  fn can_read_config_panic() {
    let mut rng = rand::rng();
    let n1: u8 = rng.random();
    env::set_var(
      ODIN_CONFIG_FILE_VAR,
      format!(
        "{}/config.{}.json",
        current_dir().unwrap().to_str().unwrap(),
        n1
      ),
    );
    read_config(config_file());
  }

  #[test]
  fn test_single_set_key() {
    let args = ValheimArguments {
      port: "2456".to_string(),
      name: "Test".to_string(),
      world: "TestWorld".to_string(),
      public: "0".to_string(),
      password: "testpass".to_string(),
      command: "/bin/echo".to_string(),
      preset: None,
      modifiers: None,
      set_key: Some("SingleKey".to_string()),
      save_interval: None,
    };

    let result: Result<Vec<String>, String> = args.try_into();
    assert!(result.is_ok());
    let args_vec = result.unwrap();

    // Find the -setkey argument
    let setkey_pos = args_vec.iter().position(|x| x == "-setkey");
    assert!(
      setkey_pos.is_some(),
      "Expected -setkey argument to be present"
    );

    let setkey_idx = setkey_pos.unwrap();
    assert_eq!(
      args_vec[setkey_idx + 1],
      "SingleKey",
      "Expected set_key value to be 'SingleKey'"
    );
  }

  #[test]
  fn test_multiple_set_keys() {
    let args = ValheimArguments {
      port: "2456".to_string(),
      name: "Test".to_string(),
      world: "TestWorld".to_string(),
      public: "0".to_string(),
      password: "testpass".to_string(),
      command: "/bin/echo".to_string(),
      preset: None,
      modifiers: None,
      set_key: Some("Key1,Key2,Key3".to_string()),
      save_interval: None,
    };

    let result: Result<Vec<String>, String> = args.try_into();
    assert!(result.is_ok());
    let args_vec = result.unwrap();

    // Count occurrences of -setkey
    let setkey_count = args_vec.iter().filter(|x| *x == "-setkey").count();
    assert_eq!(setkey_count, 3, "Expected three -setkey arguments");

    // Find all positions of -setkey
    let mut found_keys = Vec::new();
    for (i, arg) in args_vec.iter().enumerate() {
      if arg == "-setkey" && i + 1 < args_vec.len() {
        found_keys.push(args_vec[i + 1].clone());
      }
    }

    assert_eq!(
      found_keys,
      vec!["Key1", "Key2", "Key3"],
      "Expected keys to be Key1, Key2, Key3"
    );
  }

  #[test]
  fn test_multiple_set_keys_with_whitespace() {
    let args = ValheimArguments {
      port: "2456".to_string(),
      name: "Test".to_string(),
      world: "TestWorld".to_string(),
      public: "0".to_string(),
      password: "testpass".to_string(),
      command: "/bin/echo".to_string(),
      preset: None,
      modifiers: None,
      set_key: Some(" Key1 , Key2 ,  Key3  ".to_string()),
      save_interval: None,
    };

    let result: Result<Vec<String>, String> = args.try_into();
    assert!(result.is_ok());
    let args_vec = result.unwrap();

    // Find all keys
    let mut found_keys = Vec::new();
    for (i, arg) in args_vec.iter().enumerate() {
      if arg == "-setkey" && i + 1 < args_vec.len() {
        found_keys.push(args_vec[i + 1].clone());
      }
    }

    assert_eq!(
      found_keys,
      vec!["Key1", "Key2", "Key3"],
      "Expected whitespace to be trimmed"
    );
  }

  #[test]
  fn test_set_keys_with_empty_values() {
    let args = ValheimArguments {
      port: "2456".to_string(),
      name: "Test".to_string(),
      world: "TestWorld".to_string(),
      public: "0".to_string(),
      password: "testpass".to_string(),
      command: "/bin/echo".to_string(),
      preset: None,
      modifiers: None,
      set_key: Some("Key1,,Key2,  ,Key3".to_string()),
      save_interval: None,
    };

    let result: Result<Vec<String>, String> = args.try_into();
    assert!(result.is_ok());
    let args_vec = result.unwrap();

    // Find all keys
    let mut found_keys = Vec::new();
    for (i, arg) in args_vec.iter().enumerate() {
      if arg == "-setkey" && i + 1 < args_vec.len() {
        found_keys.push(args_vec[i + 1].clone());
      }
    }

    assert_eq!(
      found_keys,
      vec!["Key1", "Key2", "Key3"],
      "Expected empty values to be filtered out"
    );
  }

  #[test]
  fn test_no_set_key() {
    let args = ValheimArguments {
      port: "2456".to_string(),
      name: "Test".to_string(),
      world: "TestWorld".to_string(),
      public: "0".to_string(),
      password: "testpass".to_string(),
      command: "/bin/echo".to_string(),
      preset: None,
      modifiers: None,
      set_key: None,
      save_interval: None,
    };

    let result: Result<Vec<String>, String> = args.try_into();
    assert!(result.is_ok());
    let args_vec = result.unwrap();

    // Verify no -setkey argument exists
    let has_setkey = args_vec.iter().any(|x| x == "-setkey");
    assert!(
      !has_setkey,
      "Expected no -setkey argument when set_key is None"
    );
  }
}

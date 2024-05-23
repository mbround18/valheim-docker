use clap::{Parser, Subcommand};

use crate::utils::parse_truthy::parse_truthy;

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
pub struct Cli {
  /// Allows you to run as root
  #[arg(long, env = "I_ACCEPT_TO_RUN_THINGS_UNSAFELY", value_parser  = parse_truthy)]
  pub run_as_root: bool,

  /// Make everything noisy but very helpful to identify issues.
  /// This will enable debugging, you can use the env variable DEBUG_MODE to set this as well.
  #[arg(long, env = "DEBUG_MODE", value_parser  = parse_truthy)]
  pub debug: bool,

  /// Will spit out the commands as if it were to run them but not really.
  #[arg(short = 'r', long, env = "DRY_RUN", value_parser  = parse_truthy)]
  pub dry_run: bool,

  #[command(subcommand)]
  pub commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  /// Initializes Odin with its configuration variables.
  Configure {
    /// Sets the name of the server, (Can be set with ENV variable NAME)
    #[arg(short, long, env = "NAME")]
    #[arg(default_value_t = String::from("Valheim powered by Odin"))]
    name: String,

    /// Sets the servers executable path.
    #[arg(long, env = "SERVER_EXECUTABLE_PATH")]
    #[arg(default_value_t = format!("./{}", crate::constants::VALHEIM_EXECUTABLE_NAME))]
    server_executable: String,

    /// Sets the port of the server, (Can be set with ENV variable PORT)
    #[arg(short, long, env = "PORT")]
    #[arg(default_value_t = 2456)]
    port: u16,

    /// Sets the world of the server, (Can be set with ENV variable WORLD)
    #[arg(short, long, env = "WORLD")]
    #[arg(default_value_t = String::from("Dedicated"))]
    world: String,

    /// Sets the password of the server, (Can be set with ENV variable PASSWORD)
    #[arg(long, env = "PASSWORD")]
    #[arg(default_value_t = String::from("P@ssw0rd!"))]
    password: String,

    /// Sets the public state of the server, (Can be set with ENV variable PUBLIC)
    #[arg(short = 'o', long, env = "PUBLIC")]
    public: String,

    /// Sets flag modifiers for launching the server, (Can be set with ENV variable MODIFIERS)
    /// This should be comma separated with equal variables, e.g. "raids=none,combat=hard"
    #[arg(long, env = "MODIFIERS")]
    modifiers: Option<String>,

    /// Sets flag preset for launching the server, (Can be set with ENV variable PRESET)
    #[arg(long, env = "PRESET")]
    preset: Option<String>,

    /// Sets flag set_key for launching the server, (Can be set with ENV variable SET_KEY)
    #[arg(long, env = "SET_KEY")]
    set_key: Option<String>,

    /// Sets the save interval in seconds
    #[arg(long, env = "SAVE_INTERVAL")]
    save_interval: Option<u16>,
  },

  /// Installs Valheim with steamcmd
  Install,

  /// Starts Valheim
  Start,

  /// Stops Valheim
  Stop,

  /// Backups the current saves to a specific location
  Backup {
    /// Directory to back up
    input_directory: String,

    /// Sets the output file to use
    output_file: String,
  },

  /// Attempts to update an existing Valheim server installation. By
  /// default this involves checking for an update, if an update is
  /// available, the server will be shut down, updated, and brought back online
  /// if it was running before. If no update is available then there should
  /// be no effect from calling this.
  Update {
    /// Check for a server update, exiting with 0 if one is available and 10 if the server is up to date.
    #[arg(short, long, conflicts_with("force"), default_value_t = false)]
    check: bool,

    /// Force an update attempt, even if no update is detected.
    #[arg(short, long)]
    force: bool,
  },

  /// Sends a notification to the provided webhook.
  Notify {
    /// Title of the message block (required by discord & generic webhook, automatically supplied, default: "Broadcast")
    #[arg(long, env = "TITLE")]
    #[arg(default_value_t = String::from("Broadcast"))]
    title: String,

    /// Message to send to the webhook.
    #[arg(long, env = "MESSAGE")]
    #[arg(default_value_t = String::from("Test Notification"))]
    message: String,

    /// Sets the webhook to send a notification to, (Can be set with ENV variable WEBHOOK_URL)
    #[arg(long, env = "WEBHOOK_URL")]
    webhook_url: Option<String>,
  },

  /// Installs a mod from a given source by downloading the zip file and then extracting it.
  /// Supported platforms are Nexus (with premium account and API key), GitHub, and any other direct download source.
  #[command(name = "mod:install")]
  ModInstall {
    /// Which url you wish to pull from
    url: String,
  },

  /// Prints out the status of your server with information about current players, mod support, and a few other details.
  /// Note: If your server has PUBLIC set to 0 it will not be able to be queried!
  Status {
    /// Print out as json
    #[arg(long)]
    json: bool,

    /// Overrides address to use localhost
    #[arg(long)]
    local: bool,

    /// Search for server information based on address
    #[arg(long)]
    address: Option<String>,
  },
  /// Prints out information about Odin
  About,

  Logs {
    /// Print out as json
    #[arg(long, short = 'w')]
    watch: bool,

    /// N number of lines to print out
    #[arg(long, short = 'l', conflicts_with = "watch")]
    lines: Option<u16>,
  },
}

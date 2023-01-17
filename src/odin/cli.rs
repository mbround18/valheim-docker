use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
pub struct Cli {
  /// Allows you to run as root
  #[arg(long)]
  pub run_as_root: bool,

  /// Make everything noisy but very helpful to identify issues.
  #[arg(long)]
  pub debug: bool,

  /// Will spit out the commands as if it were to run them but not really.
  #[arg(short = 'r', long)]
  pub dry_run: bool,

  #[command(subcommand)]
  pub commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  /// Initializes Odin with its configuration variables.
  Configure {
    /// Sets the name of the server, (Can be set with ENV variable NAME)
    #[arg(short, long, default_value_t = {"Valheim powered by Odin".to_string()})]
    name: String,

    /// Sets the servers executable path.
    #[arg(long, value_name = "SERVER_EXECUTABLE_PATH", default_value_t = format!("./{}", crate::constants::VALHEIM_EXECUTABLE_NAME))]
    server_executable: String,

    /// Sets the port of the server, (Can be set with ENV variable PORT)
    #[arg(short, long, default_value_t = 2456)]
    port: u32,

    /// Sets the world of the server, (Can be set with ENV variable WORLD)
    #[arg(short, long, value_name = "WORLD", default_value_t = {"Dedicated".to_string()})]
    world: String,

    /// Sets the password of the server, (Can be set with ENV variable PASSWORD)
    #[arg(short, long, value_name = "PASSWORD", default_value_t = {"P@ssw0rd!".to_string()})]
    password: String,

    /// Sets the public state of the server, (Can be set with ENV variable PUBLIC)
    #[arg(short, long, value_name = "PUBLIC", default_value_t = false)]
    public: bool,
  },

  /// Installs Valheim with steamcmd
  Install {},

  /// Starts Valheim
  Start {},

  /// Stops Valheim
  Stop {},

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
    #[arg(short, long, default_value_t = false)]
    check: bool,

    /// Force an update attempt, even if no update is detected.
    #[arg(short, long, default_value_t = false)]
    force: bool,
  },

  /// Sends a notification to the provided webhook.
  Notify {
    /// Title of the message block (required by discord & generic webhook, automatically supplied, default: "Broadcast")
    #[arg(short, long, value_name = "TITLE", default_value_t = {"Broadcast".to_string()})]
    title: String,

    /// Message to send to the webhook.
    #[arg(short, long, value_name = "MESSAGE", default_value_t = {"Test Notification".to_string()})]
    message: String,

    /// Sets the webhook to send a notification to, (Can be set with ENV variable WEBHOOK_URL)
    #[arg(short, long, value_name = "WEBHOOK_URL")]
    webhook_url: Option<String>,
    // Sets the webhook to include the server's public IP in notifications, (Can be set with ENV variable WEBHOOK_INCLUDE_PUBLIC_IP)
    // #[arg(
    //   short,
    //   long = "webhook_include_public_ip",
    //   value_name = "WEBHOOK_INCLUDE_PUBLIC_IP"
    // )]
    // webhook_include_public_ip: bool,
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
    #[arg(short, long)]
    json: bool,

    /// Overrides address to use localhost
    #[arg(short, long)]
    local: bool,

    /// Search for server information based on address
    #[arg(short, long)]
    address: Option<String>,
  },
}

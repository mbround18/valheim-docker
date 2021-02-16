use crate::executable::find_command;
use log::{error, info};
use std::process::{exit, Command};

const STEAMCMD_EXE: &str = "/home/steam/steamcmd/steamcmd.sh";
pub fn steamcmd_command() -> Command {
    match find_command("steamcmd") {
        Some(steamcmd) => {
            info!("steamcmd found in path");
            steamcmd
        }
        None => {
            error!("Checking for script under steam user.");
            match find_command(STEAMCMD_EXE) {
                Some(steamcmd) => {
                    info!("Using steamcmd script at {}", STEAMCMD_EXE);
                    steamcmd
                }
                None => {
                    error!("\nSteamCMD Executable Not Found! \nPlease install steamcmd... \nhttps://developer.valvesoftware.com/wiki/SteamCMD\n");
                    exit(1);
                }
            }
        }
    }
}

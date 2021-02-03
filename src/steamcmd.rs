use std::process::{Command, exit};
use crate::executable::{find_command};

const STEAMCMD_EXE: &str = "/home/steam/steamcmd/steamcmd.sh";
pub fn steamcmd_command() -> Command {
    match find_command("steamcmd") {
        Some(steamcmd) => steamcmd,
        None => {
            match find_command(STEAMCMD_EXE) {
                Some(steamcmd) => steamcmd,
                None => {
                    eprint!("SteamCMD Executable Not Found! Please install steamcmd... https://developer.valvesoftware.com/wiki/SteamCMD");
                    exit(1);
                }
            }
        }
    }





}

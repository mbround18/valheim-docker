use std::process::{Command};
use crate::executable::{create_execution};

const STEAMCMD_EXE: &str = "/home/steam/steamcmd/steamcmd.sh";
pub fn steamcmd_command() -> Command {
    create_execution(STEAMCMD_EXE)
}

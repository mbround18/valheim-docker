mod steamcmd;
mod commands;
mod executable;
mod utils;

use clap::{App, load_yaml};

const GAME_ID: i64 = 896660;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();

    if let Some(ref _match) = matches.subcommand_matches("install") {
        commands::install::invoke(GAME_ID);
    };

    if let Some(ref _match) = matches.subcommand_matches("start") {
        if let Some(start_args) = matches.subcommand_matches("start") {
            commands::start::invoke(Option::from(start_args));
        } else {
            commands::start::invoke(None);
        }
    };

    if let Some(ref _match) = matches.subcommand_matches("stop") {
        commands::stop::invoke();
    };

}

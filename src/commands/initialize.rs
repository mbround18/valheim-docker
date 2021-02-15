use crate::files::config::{config_file, write_config};
use clap::ArgMatches;

pub fn invoke(args: &ArgMatches) {
    let config = config_file();
    write_config(config, args);
}

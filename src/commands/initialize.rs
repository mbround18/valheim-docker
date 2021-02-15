use crate::files::config::write_config;
use clap::ArgMatches;

pub fn invoke(args: &ArgMatches) {
    write_config(args);
}

use crate::executable::create_execution;
use crate::files::config::read_config;
use crate::utils::get_working_dir;
use clap::ArgMatches;
use daemonize::Daemonize;
use log::{error, info};
use std::fs::File;
use std::process::exit;

pub fn invoke(args: &ArgMatches) {
    info!("Setting up start scripts...");

    let config = read_config();
    if config.password.len() < 5 {
        error!("The supplied password is too short! It much be 5 characters or greater!");
        exit(1)
    }

    let dry_run: bool = args.is_present("dry_run");
    info!("Looking for burial mounds...");
    // write_rusty_start_script(&script_args, dry_run);
    if !dry_run {
        let stdout =
            File::create(format!("{}/{}", get_working_dir(), "valheim_server.out")).unwrap();
        let stderr =
            File::create(format!("{}/{}", get_working_dir(), "valheim_server.err")).unwrap();
        let daemonize = Daemonize::new()
            .working_directory(get_working_dir())
            .user("steam")
            .group("steam")
            .stdout(stdout)
            .stderr(stderr)
            .exit_action(|| {
                info!("Server has been started and Daemonized. It should be online shortly!")
            })
            .privileged_action(move || {
                create_execution(&config.command.as_str())
                    .args(&[
                        "-nographics",
                        "-port",
                        &config.port.as_str(),
                        "-name",
                        &config.name.as_str(),
                        "-world",
                        &config.world.as_str(),
                        "-password",
                        &config.password.as_str(),
                        "-public",
                        &config.public.as_str(),
                    ])
                    .spawn()
            });

        match daemonize.start() {
            Ok(_) => println!("Success, daemonized"),
            Err(e) => eprintln!("Error, {}", e),
        }
    }
}

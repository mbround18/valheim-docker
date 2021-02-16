use crate::executable::create_execution;
use crate::files::config::{config_file, read_config};
use crate::utils::{create_file, get_working_dir};
use clap::ArgMatches;
use daemonize::Daemonize;
use log::{error, info};
use std::process::exit;

pub fn invoke(args: &ArgMatches) {
    info!("Setting up start scripts...");

    let config = config_file();
    let config_content = read_config(config);
    if config_content.password.len() < 5 {
        error!("The supplied password is too short! It much be 5 characters or greater!");
        exit(1)
    }
    let dry_run: bool = args.is_present("dry_run");
    info!("Looking for burial mounds...");
    if !dry_run {
        let stdout = create_file(format!("{}/logs/valheim_server.log", get_working_dir()).as_str());
        let stderr = create_file(format!("{}/logs/valheim_server.err", get_working_dir()).as_str());
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
                create_execution(&config_content.command.as_str())
                    .args(&[
                        "-nographics",
                        "-port",
                        &config_content.port.as_str(),
                        "-name",
                        &config_content.name.as_str(),
                        "-world",
                        &config_content.world.as_str(),
                        "-password",
                        &config_content.password.as_str(),
                        "-public",
                        &config_content.public.as_str(),
                    ])
                    .spawn()
            });

        match daemonize.start() {
            Ok(_) => info!("Success, daemonized"),
            Err(e) => error!("Error, {}", e),
        }
    } else {
        info!(
            "This command would have launched\n{} -port {} -name {} -world {} -password {} -public {}",
            &config_content.command,
            &config_content.port,
            &config_content.name,
            &config_content.world,
            &config_content.password,
            &config_content.public,
        )
    }
}

use crate::files::ValheimArguments;
use crate::files::{FileManager, ManagedFile};
use crate::utils::{get_variable, get_working_dir};
use clap::ArgMatches;
use std::env;
use std::fs;
use std::path::PathBuf;

fn config_file() -> ManagedFile {
    let name = env::var("ODIN_CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    ManagedFile { name }
}

pub fn read_config() -> ValheimArguments {
    let file = config_file();
    let content = file.read();
    if content.is_empty() {
        panic!("Please initialize odin with `odin init`. See `odin init --help`")
    }
    serde_json::from_str(content.as_str()).unwrap()
}

pub fn write_config(args: &ArgMatches) -> bool {
    let file = config_file();
    let server_executable: &str =
        &[get_working_dir(), "valheim_server.x86_64".to_string()].join("/");
    let command = PathBuf::from(get_variable(
        args,
        "server_executable",
        server_executable.to_string(),
    ));
    let content = &ValheimArguments {
        port: get_variable(args, "port", "2456".to_string()),
        name: get_variable(args, "name", "Valheim powered by Odin".to_string()),
        world: get_variable(args, "world", "Dedicated".to_string()),
        public: get_variable(args, "public", "1".to_string()),
        password: get_variable(args, "password", "12345".to_string()),
        command: fs::canonicalize(&command)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    };
    file.write(serde_json::to_string(content).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::env::current_dir;

    #[test]
    #[should_panic(expected = "Please initialize odin with `odin init`. See `odin init --help`")]
    fn can_read_config_panic() {
        let mut rng = rand::thread_rng();
        let n1: u8 = rng.gen();
        env::set_var(
            "ODIN_CONFIG_PATH",
            format!(
                "{}/config.{}.json",
                current_dir().unwrap().to_str().unwrap(),
                n1
            ),
        );
        read_config();
    }
}

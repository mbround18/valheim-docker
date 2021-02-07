use crate::files::{ManagedFile, FileManager};
use log::{info};

const SERVER_EXIT: ManagedFile = ManagedFile {
    name: "server_exit.drp"
};

pub fn delete_if_exist() {
    if SERVER_EXIT.exists() {
        SERVER_EXIT.remove();
    }
}

pub fn stop_server() {
    if SERVER_EXIT.write("1".to_string()) {
        info!("Created Server Stop Waiting for it to finish.");
    };
}

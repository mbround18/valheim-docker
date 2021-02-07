use crate::files::{ManagedFile, FileManager};
use sysinfo::{System, ProcessExt};
use sysinfo::SystemExt;

const SERVER_PID: ManagedFile = ManagedFile {
    name: "valheim_server.pid"
};

pub fn read_pid() -> String {
    SERVER_PID.read()
}

pub fn is_running() -> bool {
    let pid: String = read_pid();
    if pid.len() > 0 {
        let processed_pid: i32 = pid.parse().unwrap_or(0);
        return if processed_pid != 0 {
            let system = System::new();
            let process = system.get_process(processed_pid);
            return if let Some(running_process) = process {
                running_process.pid() == processed_pid
            } else {
                false
            }
        } else {
            false
        }
    }
    false
}

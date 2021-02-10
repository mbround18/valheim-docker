use crate::files::{FileManager, ManagedFile};
use tinytemplate::TinyTemplate;
use serde::Serialize;
use log::{info, error};
use std::process::exit;

static TEMPLATE: &'static &str = &r#"
#!/usr/bin/env bash
cd "$(dirname "$0")"
# This script will be overwritten at each start!

# Launch Command
{command} \
    -port {port} \
    -name "{name}" \
    -world "{world}" \
    -password "{password}" \
    -public {public} \
    2>&1 | tee ./output.log  > /dev/null 2>&1 &

# Capture the PID
echo $! > ./valheim_server.pid
# Release the process
disown

"#;

#[derive(Serialize)]
pub struct ValheimArguments {
    pub(crate) port: String,
    pub(crate) name: String,
    pub(crate) world: String,
    pub(crate) public: String,
    pub(crate) password: String,
    pub(crate) command: String
}

pub fn write_rusty_start_script(context: &ValheimArguments, dry_run: bool) {
    let mut tt = TinyTemplate::new();
    tt.add_template(
        "hello", &TEMPLATE).unwrap();
    let content = tt
        .render("hello", &context)
        .unwrap()
        .replace("&quot;", "\"");
    let file = ManagedFile {
        name: "start_server_rusty.sh"
    };
    if dry_run {
        info!("This would have written a file to\n{}", file.path());
        info!("With contents of:\n{}", content);
    } else {
        if file.write(content) {
            info!("Created the {} script successfully!", file.path());
            if file.set_executable() {
                info!("Successfully set {} to executable!", file.path());
                return;
            };
        } else {
            error!("Failed to create file!");
            exit(1);
        };
    }
}

use flate2::write::GzEncoder;
use flate2::Compression;
use log::{debug, error};
use std::fs::File;
use std::process::exit;

pub fn invoke(input: String, output: String) {
  debug!("Creating archive of {input}");
  debug!("Output set to {output}");
  let tar_gz = match File::create(&output) {
    Ok(file) => file,
    Err(_) => {
      error!("Failed to create backup file at {}", &output);
      exit(1)
    }
  };
  let enc = GzEncoder::new(tar_gz, Compression::default());
  let mut tar = tar::Builder::new(enc);
  match tar.append_dir_all("saves", &input) {
    Ok(_) => debug!("Successfully created backup zip at {output}"),
    Err(_) => {
      error!("Failed to add {input} to backup file");
      exit(1)
    }
  };
}

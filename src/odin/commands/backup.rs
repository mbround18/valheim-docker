use flate2::write::GzEncoder;
use flate2::Compression;
use glob::glob;
use log::{debug, error, info};
use std::fs::{remove_file, File};
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

  let input_glob = glob(&format!("{input}/**/*")).expect("Failed to read glob pattern");

  for entry in input_glob {
    match entry {
      Ok(path) => {
        let name = path.display().to_string();
        if name.contains("backup_auto") {
          continue;
        }

        info!(
          "Adding {name} to backup file, with path {}",
          name.replace(&input, "")
        );

        match tar.append_path_with_name(&name, name.replace(&format!("{input}/"), "")) {
          Ok(_) => debug!("Successfully added {name} to backup file"),
          Err(err) => {
            error!("Failed to add {name} to backup file");
            error!("{:?}", err);
            remove_file(&output).unwrap();
            exit(1)
          }
        };
      }
      Err(e) => println!("{:?}", e),
    }
  }
  tar.finish().unwrap();
}

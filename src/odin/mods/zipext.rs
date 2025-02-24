use log::debug;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use zip::{
  result::{ZipError, ZipResult},
  ZipArchive,
};

pub trait ZipExt {
  fn extract_sub_dir_custom<P: AsRef<Path>>(&mut self, dst_dir: P, sub_dir: &str) -> ZipResult<()>;
}

impl ZipExt for ZipArchive<File> {
  fn extract_sub_dir_custom<P: AsRef<Path>>(&mut self, dst_dir: P, sub_dir: &str) -> ZipResult<()> {
    for i in 0..self.len() {
      let mut file = self.by_index(i)?;
      let enclosed_name = file
        .enclosed_name()
        .ok_or(ZipError::InvalidArchive("Invalid file path"))?;

      let filepath = match enclosed_name.strip_prefix(sub_dir) {
        Ok(path) => path,
        Err(_) => continue,
      };

      let mut out_path = dst_dir.as_ref().join(filepath);

      debug!("Extracting file: {:?}", out_path);

      if file.name().ends_with('/') {
        fs::create_dir_all(&out_path)?;
      } else {
        if let Some(p) = out_path.parent() {
          if !p.exists() {
            fs::create_dir_all(p)?;
          }
        }

        // Don't overwrite old cfg files
        if out_path.extension().unwrap_or_default() == "cfg" && out_path.exists() {
          debug!("File is config with already exiting destination! Adding '.new'");
          out_path = out_path.with_extension("cfg.new");
        }

        let mut outfile = File::create(&out_path)?;
        io::copy(&mut file, &mut outfile)?;
        debug!("Extracted file {:?}", out_path);
      }

      // Get and Set permissions
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = file.unix_mode() {
          fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
        }
      }
    }

    Ok(())
  }
}

use crate::constants::MODS_LOCATION;
use crate::mods::ValheimMod;
use crate::utils::environment::fetch_var;
use crate::utils::get_working_dir;
use clap::ArgMatches;
use log::error;

pub fn invoke(args: &ArgMatches) {
  let staging_location = fetch_var(
    MODS_LOCATION,
    format!("{}/mods", get_working_dir()).as_str(),
  );
  let mut valheim_mod = ValheimMod {
    url: String::from(args.value_of("URL").unwrap()),
    staging_location,
    installed: false,
    downloaded: false,
  };
  match valheim_mod.download() {
    Ok(_) => valheim_mod.install(),
    Err(message) => {
      error!("{}", message);
    }
  };
}

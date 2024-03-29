use log::info;

pub fn about(hash: &str) {
  println!("{}", "-".repeat(80));
  println!("Odin, a project built to manage your Valheim server.");
  println!("Built Ref: {hash}");
  println!("Join the Discussion!: https://github.com/mbround18/valheim-docker/discussions");
  println!("{}", "-".repeat(80));
}

pub fn modding_disclaimer() {
  info!("##########################################################################################################################");
  info!("DISCLAIMER! Modding your server can cause a lot of errors.");
  info!("Please do NOT post issue on the valheim-docker repo based on mod issues.");
  info!("By installing mods, you agree that you will do a root cause analysis to why your server is failing before you make a post.");
  info!("Modding is currently unsupported by the Valheim developers and limited support by the valheim-docker repo.");
  info!("If you have issues please contact the MOD developer FIRST based on the output logs.");
  info!("##########################################################################################################################");
}

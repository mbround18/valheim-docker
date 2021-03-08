use crate::constants::NEXUS_PREMIUM_API_KEY;
use crate::utils::environment::fetch_var;
use log::warn;
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct NexusDownloadLocation {
  pub(crate) name: String,
  pub(crate) short_name: String,

  #[serde(rename(deserialize = "uri"))]
  pub(crate) uri: String,
}

pub struct NexusURLParams {
  pub(crate) game: String,
  pub(crate) mod_id: String,
  pub(crate) file_id: String,
}

impl NexusURLParams {
  pub(crate) fn new(url: &Url) -> Result<NexusURLParams, String> {
    // https://www.nexusmods.com/valheim/mods/4?tab=files&file_id=1133
    let mut url_segments = url.path_segments().unwrap();
    let game = String::from(url_segments.next().unwrap());
    let descriptor = url_segments.next().unwrap();
    let mod_id = String::from(url_segments.next().unwrap());
    if !game.eq("valheim") || !descriptor.eq("mods") {
      Err(String::from("Invalid Nexus URL"))
    } else if let Some((_, unparsed_file_id)) = url
      .query_pairs()
      .find(|(param_name, _)| param_name.to_string().as_str().eq("file_id"))
    {
      let file_id = unparsed_file_id.to_string();
      let nexus_url_params = NexusURLParams {
        game,
        mod_id,
        file_id,
      };
      Ok(nexus_url_params)
    } else {
      Err(String::from("Failed to parse url"))
    }
  }
}

pub fn fetch_download_url(url: &Url) -> Result<String, String> {
  // Example URL: https://www.nexusmods.com/valheim/mods/4?tab=files&file_id=1133
  match NexusURLParams::new(&url) {
    Ok(url_params) => {
      let nexus_api_key = fetch_var(NEXUS_PREMIUM_API_KEY, "");
      if !nexus_api_key.is_empty() {
        let nexus_url= format!(
          "https://api.nexusmods.com//v1/games/{game}/mods/{mod_id}/files/{file_id}/download_link.json",
          game = url_params.game,
          mod_id = url_params.mod_id,
          file_id = url_params.file_id
        );
        let nexus_url_parsed = Url::parse(nexus_url.as_str()).unwrap();
        let client = reqwest::blocking::Client::new();
        if let Ok(resp) = client
          .get(nexus_url_parsed)
          .header("apikey", nexus_api_key)
          .send()
        {
          let body: Vec<NexusDownloadLocation> = resp.json().unwrap();
          let download_location = body
            .iter()
            .find(|download_locations| download_locations.name.contains("Global"))
            .unwrap();
          Ok(
            Url::parse(&download_location.uri.as_str())
              .unwrap()
              .to_string(),
          )
        } else {
          Err(String::from(
            "Request failed! Missing premium subscription to Nexus Mods...",
          ))
        }
      } else {
        Err(String::from("Nexus API key not found"))
      }
    }
    Err(message) => {
      warn!("{}", message);
      Err(message)
    }
  }
}

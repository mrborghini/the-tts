use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TwitchConfiguration {
    pub oauth_token: String,
    pub username: String,
    pub channel: String,
}

pub fn read_and_parse_config() -> Option<TwitchConfiguration> {
    // Read the file
    let contents = fs::read_to_string("config.json").ok()?;
    let config: TwitchConfiguration = serde_json::from_str(&contents).ok()?;

    Some(config)
}

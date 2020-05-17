use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub request_definition_directory: String,
    pub environment_directory: String,
    pub history_file: String,
    pub theme: Option<String>,
    pub connect_timeout_seconds: Option<u64>,
    pub read_timeout_seconds: Option<u64>,
    pub timeout_seconds: Option<u64>,
}

impl Config {
    pub fn new(path: &Path) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;

        Ok(config)
    }
}

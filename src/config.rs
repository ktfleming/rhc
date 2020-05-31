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
    pub max_history_items: Option<u64>,
    pub colors: Option<CustomColors>,
}

impl Config {
    pub fn new(path: &Path) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            request_definition_directory: "~/rhc/definitions".to_string(),
            environment_directory: "~/rhc/environments".to_string(),
            history_file: "~/.rhc_history".to_string(),
            theme: None,
            connect_timeout_seconds: None,
            read_timeout_seconds: None,
            timeout_seconds: None,
            max_history_items: None,
            colors: None,
        }
    }
}

/// Colors that the user can specify in their config file. These are just Strings and will have to
/// be parsed into tui::style::Color items. If a provided color is invalid or missing, we just fall
/// back to the default color for that item.
#[derive(Deserialize, Debug)]
pub struct CustomColors {
    pub default_fg: Option<String>,
    pub default_bg: Option<String>,
    pub selected_fg: Option<String>,
    pub selected_bg: Option<String>,
    pub prompt_fg: Option<String>,
    pub prompt_bg: Option<String>,
    pub variable_fg: Option<String>,
    pub variable_bg: Option<String>,
}

use crate::keyvalue::KeyValue;
use anyhow;
use serde::Deserialize;
use std::path::Path;
use toml;

#[derive(Deserialize, Debug)]
pub struct Environment {
    pub variables: Vec<KeyValue>,
}

impl Environment {
    pub fn new(path: &Path) -> anyhow::Result<Environment> {
        let contents = std::fs::read_to_string(path)?;

        let environment = toml::from_str(&contents)?;

        Ok(environment)
    }
}

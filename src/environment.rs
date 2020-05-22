use crate::keyvalue::KeyValue;
use anyhow::anyhow;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct Environment {
    pub name: String,
    pub variables: Vec<KeyValue>,
}

impl Environment {
    pub fn new(path: &Path) -> anyhow::Result<Environment> {
        let contents = std::fs::read_to_string(path)?;

        let environment: Environment = toml::from_str(&contents)?;

        // Disallow duplicate variable definitions
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for var in &environment.variables {
            *counts.entry(&var.name).or_insert(0) += 1;
        }

        let dupes: Vec<&str> = counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect();
        if !dupes.is_empty() {
            Err(anyhow!(
                "The specified environment file {} contains duplicate bindings for: {}",
                path.to_string_lossy(),
                dupes.join(", ")
            ))
        } else {
            Ok(environment)
        }
    }
}

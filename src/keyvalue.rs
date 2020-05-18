use serde::Deserialize;
use std::cmp::Ord;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Deserialize, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct KeyValue {
    pub name: String,
    pub value: String,
}

impl KeyValue {
    pub fn new(name: &str, value: &str) -> KeyValue {
        KeyValue {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct KeyValueParsingError;

impl Display for KeyValueParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bindings must be specified in the format 'key=value'")
    }
}

impl FromStr for KeyValue {
    type Err = KeyValueParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split('=').collect();

        match split[..] {
            [l, r] if !l.trim().is_empty() => Ok(KeyValue::new(l, r)),
            _ => Err(KeyValueParsingError),
        }
    }
}

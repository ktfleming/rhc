use serde::Deserialize;

#[derive(Deserialize, Debug)]
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

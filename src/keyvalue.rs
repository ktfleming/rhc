use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct KeyValue {
    pub name: String,
    pub value: String,
}

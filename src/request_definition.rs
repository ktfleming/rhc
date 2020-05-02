use failure::Error;
use reqwest;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use toml;

#[derive(Deserialize, Debug)]
pub struct Metadata {
    name: String,
}

#[derive(Deserialize, Debug)]
pub enum Method {
    Get,
    Post,
    Patch,
    Delete,
    Put,
    // TODO: add the rest
}

impl Method {
    pub fn to_reqwest_method(&self) -> reqwest::Method {
        match &self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
            Method::Put => reqwest::Method::PUT,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Request {
    pub url: String,
    pub method: Method,
}

#[derive(Deserialize, Debug)]
pub struct RequestDefinition {
    metadata: Metadata,
    pub request: Request,
}

impl RequestDefinition {
    pub fn new(path: &PathBuf) -> Result<RequestDefinition, Error> {
        let contents = fs::read_to_string(path)?;

        let request_def = toml::from_str(&contents)?;

        Ok(request_def)
    }
}

#[test]
fn test_new_ok() {
    RequestDefinition::new(&PathBuf::from("test_definitions/ok/ok1.toml")).unwrap();
    ()
}

#[test]
#[should_panic]
fn test_new_bad() {
    RequestDefinition::new(&PathBuf::from("test_definitions/bad/bad1.toml")).unwrap();
    ()
}

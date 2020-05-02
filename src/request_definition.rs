use anyhow;
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
pub struct Body {
    pub content_type: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub struct RequestDefinition {
    metadata: Option<Metadata>,
    pub request: Request,
    pub body: Option<Body>,
}

impl RequestDefinition {
    pub fn new(path: &PathBuf) -> anyhow::Result<RequestDefinition> {
        let contents = fs::read_to_string(path)?;

        let request_def = toml::from_str(&contents)?;

        Ok(request_def)
    }
}

#[test]
fn test_ok_files() {
    for entry in fs::read_dir("test_definitions/ok").unwrap() {
        let path = entry.unwrap().path();

        let result = RequestDefinition::new(&path);
        assert!(
            result.is_ok(),
            "expected file {:?} to be OK, but it errored",
            path.to_string_lossy()
        );
    }
}

#[test]
fn test_bad_files() {
    for entry in fs::read_dir("test_definitions/bad").unwrap() {
        let path = entry.unwrap().path();

        let result = RequestDefinition::new(&path);
        assert!(
            result.is_err(),
            "expected file {:?} to error, but it was OK",
            path.to_string_lossy()
        );
    }
}

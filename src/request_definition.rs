use crate::keyvalue::KeyValue;
use anyhow;
use attohttpc;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use toml;

#[derive(Deserialize, Debug)]
pub struct Metadata {
    name: String,
}

#[derive(Deserialize, Debug)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
    TRACE,
}

impl Method {
    pub fn to_http_method(&self) -> attohttpc::Method {
        match &self {
            Method::GET => attohttpc::Method::GET,
            Method::POST => attohttpc::Method::POST,
            Method::PUT => attohttpc::Method::PUT,
            Method::DELETE => attohttpc::Method::DELETE,
            Method::HEAD => attohttpc::Method::HEAD,
            Method::OPTIONS => attohttpc::Method::OPTIONS,
            Method::PATCH => attohttpc::Method::PATCH,
            Method::TRACE => attohttpc::Method::TRACE,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Request {
    pub url: String,
    pub method: Method,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "content")]
pub enum Content {
    Text(String),

    #[serde(with = "serde_with::json::nested")]
    Json(serde_json::Value),
}

#[derive(Deserialize, Debug)]
pub struct Headers {
    pub headers: Vec<KeyValue>,
}

#[derive(Deserialize, Debug)]
pub struct RequestDefinition {
    metadata: Option<Metadata>,
    pub request: Request,
    pub body: Option<Content>,
    pub headers: Option<Headers>,
}

impl RequestDefinition {
    pub fn new(path: &Path) -> anyhow::Result<RequestDefinition> {
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
            "expected file {:?} to be OK, but it errored with {:?}",
            path.to_string_lossy(),
            result
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
            "expected file {:?} to error, but it was OK. Got result {:?}",
            path.to_string_lossy(),
            result
        );
    }
}

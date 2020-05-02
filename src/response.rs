use reqwest;
use std::fmt;

#[derive(Debug)]
pub struct Response {
    pub body: String,
    pub status_code: reqwest::StatusCode,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}", self.status_code, self.body)
    }
}

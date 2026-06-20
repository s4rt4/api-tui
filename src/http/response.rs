use reqwest::header::HeaderMap;
use std::time::Duration;

pub struct Response {
    pub status: u16,
    pub elapsed: Duration,
    pub headers: HeaderMap,
    pub body: String,
}

impl Response {
    pub fn size_bytes(&self) -> usize {
        self.body.len()
    }

    pub fn status_class(&self) -> StatusClass {
        match self.status {
            100..=199 => StatusClass::Info,
            200..=299 => StatusClass::Success,
            300..=399 => StatusClass::Redirect,
            400..=499 => StatusClass::ClientError,
            500..=599 => StatusClass::ServerError,
            _ => StatusClass::Unknown,
        }
    }

    /// True when the `Content-Type` advertises JSON.
    pub fn is_json(&self) -> bool {
        self.headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("application/json"))
            .unwrap_or(false)
    }

    /// Body pretty-printed when it is valid JSON, otherwise returned verbatim.
    pub fn pretty_body(&self) -> String {
        if self.is_json() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&self.body) {
                if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                    return pretty;
                }
            }
        }
        self.body.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusClass {
    Info,
    Success,
    Redirect,
    ClientError,
    ServerError,
    Unknown,
}

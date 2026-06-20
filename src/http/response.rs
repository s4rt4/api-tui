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

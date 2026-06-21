use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Collection {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub requests: Vec<Request>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub query: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Body {
    #[serde(rename = "type", default = "default_body_type")]
    pub kind: String,
    #[serde(default)]
    pub content: String,
    /// Fields for a `multipart` body. Ignored for other kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<MultipartPart>,
}

/// One field of a `multipart/form-data` body: either a text value or a file
/// upload referenced by path.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultipartPart {
    pub name: String,
    /// Text field value. Used when `file` is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Path to a file to upload as this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional filename to advertise (defaults to the file's own name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional per-part Content-Type.
    #[serde(
        rename = "content_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub content_type: Option<String>,
}

fn default_body_type() -> String {
    "raw".into()
}

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiTesterError {
    #[error("collection file not found: {0}")]
    CollectionNotFound(PathBuf),

    #[error("invalid TOML: {0}")]
    TomlParse(String),

    #[error("invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("undefined variable: {{{{{0}}}}}")]
    UndefinedVar(String),

    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml decode error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

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

    #[error("{}", classify_reqwest(.0))]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("cannot read upload file '{path}': {source}")]
    FileRead {
        path: String,
        source: std::io::Error,
    },

    #[error("toml decode error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

/// Turn an opaque `reqwest::Error` into an actionable, human-readable message.
/// Walks the source chain so root causes (DNS, TLS, refused) surface even when
/// reqwest only reports a generic category.
fn classify_reqwest(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        return "request timed out (raise --timeout if the server is just slow)".to_string();
    }

    let chain = error_chain(e);
    let low = chain.to_lowercase();

    if low.contains("certificate")
        || low.contains("tls")
        || low.contains("handshake")
        || low.contains("self-signed")
        || low.contains("self signed")
        || low.contains("unknownissuer")
    {
        return format!(
            "TLS error: {chain} (pass --insecure to skip verification for trusted hosts)"
        );
    }

    if e.is_connect() {
        if low.contains("dns")
            || low.contains("resolve")
            || low.contains("lookup")
            || low.contains("name or service not known")
            || low.contains("no such host")
        {
            return format!("DNS resolution failed: {chain}");
        }
        if low.contains("refused") {
            return format!(
                "connection refused: {chain} (is the server running on that host/port?)"
            );
        }
        return format!("connection failed: {chain}");
    }

    if e.is_redirect() {
        return format!("too many redirects: {chain}");
    }

    format!("request failed: {chain}")
}

/// Join an error and its `source()` chain into a single `a: b: c` string.
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut parts = Vec::new();
    let mut current: Option<&dyn std::error::Error> = Some(e);
    while let Some(err) = current {
        parts.push(err.to_string());
        current = err.source();
    }
    // Drop consecutive duplicates so the message stays readable.
    parts.dedup();
    parts.join(": ")
}

pub mod response;

use crate::error::ApiTesterError;
use reqwest::Method;
use std::time::{Duration, Instant};

pub use response::{Response, StatusClass};

#[derive(Clone)]
pub struct SendOpts {
    pub timeout: Duration,
    pub insecure: bool,
    pub follow_redirects: bool,
    pub proxy: Option<String>,
    /// Shared cookie jar; when present, cookies are sent and captured.
    pub cookies: Option<crate::cookies::Jar>,
}

impl Default for SendOpts {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            insecure: false,
            follow_redirects: true,
            proxy: None,
            cookies: None,
        }
    }
}

/// The effective request body to send: a raw text payload, or a multipart form.
#[derive(Debug, Clone)]
pub enum ReqBody {
    Text(String),
    Multipart(Vec<Part>),
}

/// One resolved multipart field.
#[derive(Debug, Clone)]
pub struct Part {
    pub name: String,
    pub kind: PartKind,
}

#[derive(Debug, Clone)]
pub enum PartKind {
    Text(String),
    File {
        path: String,
        filename: Option<String>,
        content_type: Option<String>,
    },
}

pub async fn send(
    method: &str,
    url: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&ReqBody>,
    opts: &SendOpts,
) -> Result<Response, ApiTesterError> {
    let method_upper = method.to_ascii_uppercase();
    let method_parsed = Method::from_bytes(method_upper.as_bytes())
        .map_err(|_| ApiTesterError::InvalidMethod(method.to_string()))?;

    let mut builder = reqwest::Client::builder()
        .timeout(opts.timeout)
        .danger_accept_invalid_certs(opts.insecure)
        .redirect(if opts.follow_redirects {
            reqwest::redirect::Policy::default()
        } else {
            reqwest::redirect::Policy::none()
        });

    // An explicit --proxy wins; otherwise reqwest still honors the standard
    // HTTP_PROXY / HTTPS_PROXY / NO_PROXY environment variables by default
    // (we never call .no_proxy(), so env-based proxying works out of the box).
    if let Some(proxy_url) = &opts.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }

    if let Some(jar) = &opts.cookies {
        builder = builder.cookie_provider(jar.clone());
    }

    let client = builder.build()?;

    let mut req = client.request(method_parsed, url);
    if !query.is_empty() {
        req = req.query(query);
    }
    for (k, v) in headers {
        req = req.header(k, v);
    }
    match body {
        Some(ReqBody::Text(text)) => {
            req = req.body(text.clone());
        }
        Some(ReqBody::Multipart(parts)) => {
            req = req.multipart(build_form(parts)?);
        }
        None => {}
    }

    let start = Instant::now();
    let res = req.send().await?;
    let elapsed = start.elapsed();
    let status = res.status().as_u16();
    let resp_headers = res.headers().clone();
    let body = res.text().await?;

    Ok(Response {
        status,
        elapsed,
        headers: resp_headers,
        body,
    })
}

/// Assemble a `multipart/form-data` form, reading any referenced files. reqwest
/// sets the `Content-Type` (with boundary) itself, so callers must not.
fn build_form(parts: &[Part]) -> Result<reqwest::multipart::Form, ApiTesterError> {
    let mut form = reqwest::multipart::Form::new();
    for part in parts {
        match &part.kind {
            PartKind::Text(value) => {
                form = form.text(part.name.clone(), value.clone());
            }
            PartKind::File {
                path,
                filename,
                content_type,
            } => {
                let bytes = std::fs::read(path).map_err(|e| ApiTesterError::FileRead {
                    path: path.clone(),
                    source: e,
                })?;
                let name = filename.clone().unwrap_or_else(|| file_name_of(path));
                let mut fp = reqwest::multipart::Part::bytes(bytes).file_name(name);
                if let Some(ct) = content_type {
                    fp = fp.mime_str(ct)?;
                }
                form = form.part(part.name.clone(), fp);
            }
        }
    }
    Ok(form)
}

/// The final path component, or the whole string if it has no separator.
fn file_name_of(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

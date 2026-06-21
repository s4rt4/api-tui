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

pub async fn send(
    method: &str,
    url: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&str>,
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
    if let Some(b) = body {
        req = req.body(b.to_owned());
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

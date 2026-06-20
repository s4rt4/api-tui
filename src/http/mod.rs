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
}

impl Default for SendOpts {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            insecure: false,
            follow_redirects: true,
            proxy: None,
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

    if let Some(proxy_url) = &opts.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
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

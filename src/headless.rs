//! Non-interactive mode: run a single named request, print the response, exit.
//!
//! Exit codes: `0` success (status < 400), `1` HTTP error status (>= 400),
//! `2` no collection given, `3` request name not found, `4` transport error.
//! Diagnostics go to stderr; the (pretty-printed) response body goes to stdout.

use crate::collection::{self, build};
use crate::config::Cli;
use crate::http::{self, SendOpts};
use anyhow::Result;

pub async fn run(cli: &Cli, name: &str) -> Result<i32> {
    let Some(path) = &cli.collection else {
        eprintln!("error: --headless requires a collection file (pass a .toml path)");
        return Ok(2);
    };

    let coll = collection::load(path)?;

    let Some(req) = coll.requests.iter().find(|r| r.name == name).cloned() else {
        eprintln!("error: request '{}' not found", name);
        if !coll.requests.is_empty() {
            let names: Vec<&str> = coll.requests.iter().map(|r| r.name.as_str()).collect();
            eprintln!("available: {}", names.join(", "));
        }
        return Ok(3);
    };

    let env_vars = build::resolve_env(&coll, &cli.env);
    let built = build::build_effective(&req, coll.base_url.as_deref(), &env_vars)?;

    let opts = SendOpts {
        timeout: cli.timeout_duration(),
        insecure: cli.insecure,
        follow_redirects: !cli.no_redirect,
        proxy: cli.proxy.clone(),
    };

    eprintln!("→ {} {}", built.method, built.url);

    match http::send(
        &built.method,
        &built.url,
        &built.query,
        &built.headers,
        built.body.as_deref(),
        &opts,
    )
    .await
    {
        Ok(resp) => {
            eprintln!(
                "← {} in {}ms ({} bytes)",
                resp.status,
                resp.elapsed.as_millis(),
                resp.size_bytes()
            );
            println!("{}", resp.pretty_body());
            Ok(if resp.status < 400 { 0 } else { 1 })
        }
        Err(e) => {
            eprintln!("✗ {}", e);
            Ok(4)
        }
    }
}

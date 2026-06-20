use apitester::config::Cli;
use apitester::headless;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cli(collection: PathBuf, name: &str) -> Cli {
    Cli {
        collection: Some(collection),
        env: "default".into(),
        timeout: 5,
        insecure: false,
        no_redirect: false,
        proxy: None,
        no_color: true,
        headless: Some(name.into()),
    }
}

/// Write a one-request collection to a unique temp file; returns its path.
fn write_collection(tag: &str, toml: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("apitester-headless-{}-{}.toml", std::process::id(), tag));
    std::fs::write(&p, toml).unwrap();
    p
}

#[tokio::test]
async fn success_returns_zero() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .mount(&server)
        .await;

    let toml = format!(
        "base_url = \"{}\"\n\n[[requests]]\nname = \"ping\"\nmethod = \"GET\"\nurl = \"/ping\"\n",
        server.uri()
    );
    let p = write_collection("ok", &toml);
    let code = headless::run(&cli(p.clone(), "ping"), "ping").await.unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(code, 0);
}

#[tokio::test]
async fn http_error_status_returns_one() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let toml = format!(
        "base_url = \"{}\"\n\n[[requests]]\nname = \"r\"\nmethod = \"GET\"\nurl = \"/missing\"\n",
        server.uri()
    );
    let p = write_collection("404", &toml);
    let code = headless::run(&cli(p.clone(), "r"), "r").await.unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(code, 1);
}

#[tokio::test]
async fn unknown_request_returns_three() {
    let toml = "[[requests]]\nname = \"exists\"\nmethod = \"GET\"\nurl = \"http://localhost/x\"\n";
    let p = write_collection("notfound", toml);
    let code = headless::run(&cli(p.clone(), "nope"), "nope").await.unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(code, 3);
}

#[tokio::test]
async fn transport_error_returns_four() {
    // Port 1 is not listening — connection refused → transport error.
    let toml =
        "[[requests]]\nname = \"dead\"\nmethod = \"GET\"\nurl = \"http://127.0.0.1:1/x\"\n";
    let p = write_collection("transport", toml);
    let code = headless::run(&cli(p.clone(), "dead"), "dead").await.unwrap();
    let _ = std::fs::remove_file(&p);
    assert_eq!(code, 4);
}

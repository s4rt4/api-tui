//! End-to-end cookie jar test: cookies set by a response are sent on later
//! requests sharing the jar, absent without a jar, and survive a save/reload
//! round-trip.
//!
//! Run as a single sequential test: `APITESTER_DATA_DIR` is process-global, so
//! splitting these into concurrent tests would race on it.

use apitester::cookies;
use apitester::http::{self, SendOpts};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn opts(jar: cookies::Jar) -> SendOpts {
    SendOpts {
        cookies: Some(jar),
        ..SendOpts::default()
    }
}

async fn get(server: &MockServer, ep: &str, opts: &SendOpts) -> u16 {
    http::send(
        "GET",
        &format!("{}{}", server.uri(), ep),
        &[],
        &[],
        None,
        opts,
    )
    .await
    .unwrap()
    .status
}

#[tokio::test]
async fn cookie_jar_end_to_end() {
    let mut dir = std::env::temp_dir();
    dir.push(format!("apitester-cookie-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("APITESTER_DATA_DIR", &dir);

    let server = MockServer::start().await;
    // /set hands out a persistent cookie.
    Mock::given(method("GET"))
        .and(path("/set"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=abc; Max-Age=3600; Path=/"),
        )
        .mount(&server)
        .await;
    // /check returns 200 only when the cookie is presented, else 400.
    Mock::given(method("GET"))
        .and(path("/check"))
        .and(header("cookie", "session=abc"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/check"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    // Acquire the cookie, then a request sharing the jar carries it.
    let jar = cookies::load();
    assert_eq!(get(&server, "/set", &opts(jar.clone())).await, 200);
    assert_eq!(get(&server, "/check", &opts(jar.clone())).await, 200);

    // Without a jar, no cookie is sent.
    assert_eq!(get(&server, "/check", &SendOpts::default()).await, 400);

    // After save + reload, a fresh jar from disk still carries the cookie.
    cookies::save(&jar);
    let reloaded = cookies::load();
    assert_eq!(get(&server, "/check", &opts(reloaded)).await, 200);
}

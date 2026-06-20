use apitester::http::{send, SendOpts};
use std::time::Duration;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn opts() -> SendOpts {
    SendOpts {
        timeout: Duration::from_secs(5),
        ..Default::default()
    }
}

#[tokio::test]
async fn get_returns_200_with_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("world"))
        .mount(&server)
        .await;

    let url = format!("{}/hello", server.uri());
    let resp = send("GET", &url, &[], &[], None, &opts()).await.unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, "world");
}

#[tokio::test]
async fn post_with_body_and_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/echo"))
        .and(header("X-Test", "yes"))
        .and(body_string("hi"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let url = format!("{}/echo", server.uri());
    let resp = send(
        "POST",
        &url,
        &[],
        &[("X-Test".into(), "yes".into())],
        Some("hi"),
        &opts(),
    )
    .await
    .unwrap();
    assert_eq!(resp.status, 201);
}

#[tokio::test]
async fn query_params_encoded() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/q"))
        .and(query_param("page", "2"))
        .and(query_param("q", "hello world"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let url = format!("{}/q", server.uri());
    let resp = send(
        "GET",
        &url,
        &[
            ("page".into(), "2".into()),
            ("q".into(), "hello world".into()),
        ],
        &[],
        None,
        &opts(),
    )
    .await
    .unwrap();
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn lowercase_method_uppercased() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let url = format!("{}/x", server.uri());
    let resp = send("delete", &url, &[], &[], None, &opts()).await.unwrap();
    assert_eq!(resp.status, 204);
}

#[tokio::test]
async fn invalid_method_returns_error() {
    let result = send("BAD METHOD", "http://localhost", &[], &[], None, &opts()).await;
    assert!(matches!(
        result,
        Err(apitester::error::ApiTesterError::InvalidMethod(_))
    ));
}

#[tokio::test]
async fn captures_response_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/h"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("X-Custom", "abc")
                .set_body_string("{}"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/h", server.uri());
    let resp = send("GET", &url, &[], &[], None, &opts()).await.unwrap();
    assert_eq!(resp.headers.get("x-custom").unwrap(), "abc");
}

#[tokio::test]
async fn measures_elapsed_time() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(50)))
        .mount(&server)
        .await;

    let url = format!("{}/slow", server.uri());
    let resp = send("GET", &url, &[], &[], None, &opts()).await.unwrap();
    assert!(resp.elapsed >= Duration::from_millis(50));
}

#[tokio::test]
async fn timeout_has_friendly_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hang"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
        .mount(&server)
        .await;

    let url = format!("{}/hang", server.uri());
    let fast = SendOpts {
        timeout: Duration::from_millis(150),
        ..Default::default()
    };
    let err = send("GET", &url, &[], &[], None, &fast).await.unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("timed out"),
        "got: {err}"
    );
}

#[tokio::test]
async fn connection_refused_has_friendly_message() {
    // Port 1 is reserved and never listening → connection refused.
    let err = send("GET", "http://127.0.0.1:1/x", &[], &[], None, &opts())
        .await
        .unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("refused") || msg.contains("connection failed"),
        "got: {msg}"
    );
}

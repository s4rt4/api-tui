use apitester::collection;
use std::path::Path;

#[test]
fn loads_example_collection() {
    let path = Path::new("collections/example.toml");
    let coll = collection::load(path).expect("load example collection");
    assert_eq!(coll.name.as_deref(), Some("Example API"));
    assert_eq!(coll.base_url.as_deref(), Some("https://httpbin.org"));
    assert_eq!(coll.requests.len(), 3);
    assert_eq!(coll.requests[0].name, "Get Status");
    assert_eq!(coll.requests[0].method, "GET");
}

#[test]
fn missing_file_returns_not_found() {
    let path = Path::new("collections/does-not-exist.toml");
    let err = collection::load(path).expect_err("should error");
    assert!(matches!(
        err,
        apitester::error::ApiTesterError::CollectionNotFound(_)
    ));
}

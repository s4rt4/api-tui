use crate::collection::interpolate::interpolate;
use crate::collection::model::{Collection, Request};
use crate::error::ApiTesterError;
use std::collections::HashMap;

pub struct EffectiveRequest {
    pub method: String,
    pub url: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Merge `[env.default]` then `[env.<name>]` (active overrides default).
pub fn resolve_env(coll: &Collection, env_name: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    if let Some(default) = coll.env.get("default") {
        vars.extend(default.clone());
    }
    if env_name != "default" {
        if let Some(specific) = coll.env.get(env_name) {
            vars.extend(specific.clone());
        }
    }
    vars
}

pub fn build_effective(
    req: &Request,
    base_url: Option<&str>,
    env_vars: &HashMap<String, String>,
) -> Result<EffectiveRequest, ApiTesterError> {
    let url_resolved = interpolate(&req.url, env_vars)?;
    let url = resolve_url(&url_resolved, base_url);

    let mut headers = Vec::with_capacity(req.headers.len());
    let mut hkeys: Vec<_> = req.headers.keys().collect();
    hkeys.sort();
    for k in hkeys {
        let v = interpolate(&req.headers[k], env_vars)?;
        headers.push((k.clone(), v));
    }

    let mut query = Vec::with_capacity(req.query.len());
    let mut qkeys: Vec<_> = req.query.keys().collect();
    qkeys.sort();
    for k in qkeys {
        let v = interpolate(&req.query[k], env_vars)?;
        query.push((k.clone(), v));
    }

    let body = match &req.body {
        Some(b) => Some(interpolate(&b.content, env_vars)?),
        None => None,
    };

    // Derive a Content-Type from the body's declared `kind` (json/form/xml/text)
    // unless the request already sets one explicitly.
    if body.is_some() {
        let has_ct = headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
        if !has_ct {
            if let Some(ct) = content_type_for(req.body.as_ref().map(|b| b.kind.as_str())) {
                headers.push(("Content-Type".to_string(), ct.to_string()));
            }
        }
    }

    Ok(EffectiveRequest {
        method: req.method.to_ascii_uppercase(),
        url,
        query,
        headers,
        body,
    })
}

/// Map a body `kind` to its conventional Content-Type.
fn content_type_for(kind: Option<&str>) -> Option<&'static str> {
    match kind.map(|k| k.to_ascii_lowercase()).as_deref() {
        Some("json") => Some("application/json"),
        Some("form") => Some("application/x-www-form-urlencoded"),
        Some("xml") => Some("application/xml"),
        Some("text") => Some("text/plain"),
        _ => None,
    }
}

fn resolve_url(url: &str, base_url: Option<&str>) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    match base_url {
        Some(base) => {
            let base = base.trim_end_matches('/');
            if url.starts_with('/') {
                format!("{}{}", base, url)
            } else {
                format!("{}/{}", base, url)
            }
        }
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, url: &str) -> Request {
        Request {
            name: "t".into(),
            method: method.into(),
            url: url.into(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: None,
        }
    }

    #[test]
    fn absolute_url_passthrough() {
        let r = req("GET", "https://api.example.com/foo");
        let built = build_effective(&r, Some("https://other.com"), &HashMap::new()).unwrap();
        assert_eq!(built.url, "https://api.example.com/foo");
    }

    #[test]
    fn relative_url_with_base() {
        let r = req("GET", "/users");
        let built = build_effective(&r, Some("https://api.example.com"), &HashMap::new()).unwrap();
        assert_eq!(built.url, "https://api.example.com/users");
    }

    #[test]
    fn base_url_trailing_slash_normalized() {
        let r = req("GET", "/users");
        let built = build_effective(&r, Some("https://api.example.com/"), &HashMap::new()).unwrap();
        assert_eq!(built.url, "https://api.example.com/users");
    }

    #[test]
    fn relative_url_no_base() {
        let r = req("GET", "/users");
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        assert_eq!(built.url, "/users");
    }

    #[test]
    fn interpolates_url_headers_query_body() {
        let mut r = req("GET", "/u/{{id}}");
        r.headers
            .insert("Authorization".into(), "Bearer {{token}}".into());
        r.query.insert("page".into(), "{{page}}".into());
        r.body = Some(crate::collection::model::Body {
            kind: "json".into(),
            content: "{\"x\": \"{{val}}\"}".into(),
        });
        let mut env = HashMap::new();
        env.insert("id".into(), "42".into());
        env.insert("token".into(), "abc".into());
        env.insert("page".into(), "1".into());
        env.insert("val".into(), "hello".into());
        let built = build_effective(&r, Some("https://x.com"), &env).unwrap();
        assert_eq!(built.url, "https://x.com/u/42");
        // Authorization (user) plus an auto Content-Type from the json body kind.
        assert_eq!(
            built.headers,
            vec![
                ("Authorization".into(), "Bearer abc".into()),
                ("Content-Type".into(), "application/json".into()),
            ]
        );
        assert_eq!(built.query, vec![("page".into(), "1".into())]);
        assert_eq!(built.body.as_deref(), Some("{\"x\": \"hello\"}"));
    }

    fn req_with_body(kind: &str, content: &str) -> Request {
        let mut r = req("POST", "/x");
        r.body = Some(crate::collection::model::Body {
            kind: kind.into(),
            content: content.into(),
        });
        r
    }

    #[test]
    fn form_body_gets_urlencoded_content_type() {
        let r = req_with_body("form", "a=1&b=2");
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        assert!(built.headers.contains(&(
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into()
        )));
    }

    #[test]
    fn explicit_content_type_is_not_overridden() {
        let mut r = req_with_body("json", "{}");
        r.headers
            .insert("Content-Type".into(), "application/vnd.api+json".into());
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        let cts: Vec<_> = built
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .collect();
        assert_eq!(cts.len(), 1);
        assert_eq!(cts[0].1, "application/vnd.api+json");
    }

    #[test]
    fn no_body_means_no_content_type() {
        let r = req("GET", "/x");
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        assert!(!built
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type")));
    }

    #[test]
    fn unknown_body_kind_adds_no_content_type() {
        let r = req_with_body("binary", "stuff");
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        assert!(!built
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type")));
    }

    #[test]
    fn method_uppercased() {
        let r = req("get", "/foo");
        let built = build_effective(&r, None, &HashMap::new()).unwrap();
        assert_eq!(built.method, "GET");
    }

    #[test]
    fn resolve_env_default_only() {
        let mut coll = Collection::default();
        let mut def = HashMap::new();
        def.insert("a".into(), "1".into());
        coll.env.insert("default".into(), def);
        let v = resolve_env(&coll, "default");
        assert_eq!(v.get("a"), Some(&"1".to_string()));
    }

    #[test]
    fn resolve_env_overrides_default() {
        let mut coll = Collection::default();
        let mut def = HashMap::new();
        def.insert("token".into(), "dev".into());
        def.insert("only_in_default".into(), "x".into());
        coll.env.insert("default".into(), def);
        let mut prod = HashMap::new();
        prod.insert("token".into(), "prod".into());
        coll.env.insert("prod".into(), prod);
        let v = resolve_env(&coll, "prod");
        assert_eq!(v.get("token"), Some(&"prod".to_string()));
        assert_eq!(v.get("only_in_default"), Some(&"x".to_string()));
    }

    #[test]
    fn missing_env_section_returns_empty() {
        let coll = Collection::default();
        let v = resolve_env(&coll, "nonexistent");
        assert!(v.is_empty());
    }
}

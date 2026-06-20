use crate::error::ApiTesterError;
use std::collections::HashMap;

/// Substitute `{{var}}` in `input` using `vars`, falling back to OS env vars.
pub fn interpolate(input: &str, vars: &HashMap<String, String>) -> Result<String, ApiTesterError> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        match after_open.find("}}") {
            Some(end) => {
                let key = after_open[..end].trim();
                let val = vars
                    .get(key)
                    .cloned()
                    .or_else(|| std::env::var(key).ok())
                    .ok_or_else(|| ApiTesterError::UndefinedVar(key.to_string()))?;
                out.push_str(&val);
                rest = &after_open[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn substitutes_known_var() {
        let v = vars(&[("token", "abc123")]);
        assert_eq!(interpolate("Bearer {{token}}", &v).unwrap(), "Bearer abc123");
    }

    #[test]
    fn handles_multiple_vars() {
        let v = vars(&[("a", "1"), ("b", "2")]);
        assert_eq!(interpolate("{{a}}-{{b}}-{{a}}", &v).unwrap(), "1-2-1");
    }

    #[test]
    fn trims_whitespace_in_key() {
        let v = vars(&[("token", "abc")]);
        assert_eq!(interpolate("{{ token }}", &v).unwrap(), "abc");
    }

    #[test]
    fn errors_on_undefined() {
        let v = HashMap::new();
        assert!(matches!(
            interpolate("Bearer {{missing}}", &v),
            Err(ApiTesterError::UndefinedVar(_))
        ));
    }

    #[test]
    fn passthrough_without_vars() {
        let v = HashMap::new();
        assert_eq!(interpolate("plain text", &v).unwrap(), "plain text");
    }

    #[test]
    fn unclosed_braces_passthrough() {
        let v = HashMap::new();
        assert_eq!(interpolate("oops {{ no end", &v).unwrap(), "oops {{ no end");
    }
}

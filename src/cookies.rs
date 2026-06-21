//! Persistent cookie jar. Cookies set by responses are stored in a shared,
//! thread-safe jar that reqwest consults on later requests, and the jar is
//! serialized to `cookies.json` in the data dir so it survives across runs.

use cookie_store::serde::json;
use reqwest_cookie_store::CookieStoreMutex;
use std::path::PathBuf;
use std::sync::Arc;

/// A shareable cookie jar; cloning is cheap (an `Arc`) and shares one store.
pub type Jar = Arc<CookieStoreMutex>;

fn cookies_path() -> Option<PathBuf> {
    crate::paths::data_file("cookies.json")
}

/// Load the persisted jar, or an empty one if there is no file (or it is
/// unreadable / malformed).
pub fn load() -> Jar {
    let store = cookies_path()
        .and_then(|p| std::fs::File::open(p).ok())
        .map(std::io::BufReader::new)
        .and_then(|r| json::load(r).ok())
        .unwrap_or_default();
    Arc::new(CookieStoreMutex::new(store))
}

/// Best-effort: persist the (unexpired, persistent) cookies to disk.
pub fn save(jar: &Jar) {
    let Some(path) = cookies_path() else {
        return;
    };
    let Ok(store) = jar.lock() else {
        return;
    };
    if let Ok(mut file) = std::fs::File::create(&path) {
        let _ = json::save(&store, &mut file);
    }
}

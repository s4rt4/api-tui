//! Resolution of the application data directory (request history, cookie jar,
//! …). Honors the `APITESTER_DATA_DIR` override (used by tests and to relocate
//! state), otherwise the platform data dir.

use std::path::PathBuf;

/// The data directory, creating it if needed. `None` if none can be determined.
pub fn data_dir() -> Option<PathBuf> {
    let dir = match std::env::var_os("APITESTER_DATA_DIR") {
        Some(d) => PathBuf::from(d),
        None => directories::ProjectDirs::from("", "", "apitester")?
            .data_dir()
            .to_path_buf(),
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    Some(dir)
}

/// Path to `name` inside the data directory (directory created if needed).
pub fn data_file(name: &str) -> Option<PathBuf> {
    Some(data_dir()?.join(name))
}

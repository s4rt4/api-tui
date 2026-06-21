//! Persistent request history: every sent request is appended as one JSON line
//! to `history.jsonl` in the platform data directory, so past calls survive
//! across runs and can be reviewed from the TUI (`H`).

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One recorded send. `status`/`elapsed_ms` are present on a completed response;
/// `error` is present instead when the transport failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub ts_ms: u64,
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

/// Milliseconds since the Unix epoch, or 0 if the clock is before it.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Path to the history file, creating its directory if needed. `None` if no
/// data directory can be determined. See [`crate::paths`] for the location and
/// the `APITESTER_DATA_DIR` override.
pub fn history_path() -> Option<PathBuf> {
    crate::paths::data_file("history.jsonl")
}

/// Best-effort: append `entry` to the history file. Errors are swallowed since
/// history is non-critical, but returned by [`append_to`] for testing.
pub fn record(entry: &HistoryEntry) {
    if let Some(path) = history_path() {
        let _ = append_to(&path, entry);
    }
}

/// The most recent `n` entries, newest first. Empty if the file is missing.
pub fn recent(n: usize) -> Vec<HistoryEntry> {
    match history_path() {
        Some(path) => recent_from(&path, n),
        None => Vec::new(),
    }
}

fn append_to(path: &Path, entry: &HistoryEntry) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)
}

fn recent_from(path: &Path, n: usize) -> Vec<HistoryEntry> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut entries: Vec<HistoryEntry> = std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect();
    entries.reverse(); // newest first
    entries.truncate(n);
    entries
}

/// Format epoch milliseconds as a UTC `YYYY-MM-DD HH:MM:SS` string.
pub fn format_ts(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}")
}

/// Convert a count of days since the Unix epoch into a (year, month, day)
/// civil date. Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, status: Option<u16>) -> HistoryEntry {
        HistoryEntry {
            ts_ms: 0,
            name: name.into(),
            method: "GET".into(),
            url: "http://x/".into(),
            status,
            elapsed_ms: Some(12),
            error: None,
        }
    }

    fn temp_file(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "apitester-hist-{}-{}.jsonl",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn append_then_read_newest_first() {
        let path = temp_file("rw");
        append_to(&path, &entry("first", Some(200))).unwrap();
        append_to(&path, &entry("second", Some(404))).unwrap();
        let got = recent_from(&path, 10);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "second");
        assert_eq!(got[1].name, "first");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recent_truncates_to_n() {
        let path = temp_file("trunc");
        for i in 0..5 {
            append_to(&path, &entry(&format!("r{i}"), Some(200))).unwrap();
        }
        let got = recent_from(&path, 2);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "r4");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_yields_empty() {
        let path = temp_file("missing");
        assert!(recent_from(&path, 10).is_empty());
    }

    #[test]
    fn bad_lines_are_skipped() {
        let path = temp_file("bad");
        std::fs::write(
            &path,
            "not json\n{\"ts_ms\":0,\"name\":\"ok\",\"method\":\"GET\",\"url\":\"u\"}\n",
        )
        .unwrap();
        let got = recent_from(&path, 10);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "ok");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn format_ts_known_epoch() {
        // 2021-01-01 00:00:00 UTC = 1_609_459_200 s
        assert_eq!(format_ts(1_609_459_200_000), "2021-01-01 00:00:00");
        // epoch
        assert_eq!(format_ts(0), "1970-01-01 00:00:00");
        // 2026-06-21 12:34:56 UTC = 1_782_045_296 s
        assert_eq!(format_ts(1_782_045_296_000), "2026-06-21 12:34:56");
    }
}

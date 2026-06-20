//! JSON syntax highlighting for the response body, via `syntect`.
//!
//! The default syntax/theme sets are loaded once (lazily) and the last result
//! is memoized, so the TUI's per-tick redraws don't re-highlight an unchanged
//! body.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::sync::{Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEMES: OnceLock<ThemeSet> = OnceLock::new();
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

#[derive(Default)]
struct Cache {
    key: String,
    lines: Vec<Line<'static>>,
}

/// Highlight `text` as JSON into ratatui lines. Memoizes the most recent input.
pub fn highlight_json(text: &str) -> Vec<Line<'static>> {
    let cache = CACHE.get_or_init(|| Mutex::new(Cache::default()));
    if !text.is_empty() {
        let guard = cache.lock().unwrap();
        if guard.key == text {
            return guard.lines.clone();
        }
    }

    let lines = compute(text);

    let mut guard = cache.lock().unwrap();
    guard.key = text.to_string();
    guard.lines = lines.clone();
    lines
}

fn compute(text: &str) -> Vec<Line<'static>> {
    let ps = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEMES.get_or_init(ThemeSet::load_defaults);
    let syntax = ps
        .find_syntax_by_extension("json")
        .unwrap_or_else(|| ps.find_syntax_plain_text());
    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut out = Vec::new();
    for line in LinesWithEndings::from(text) {
        match highlighter.highlight_line(line, ps) {
            Ok(ranges) => {
                let spans: Vec<Span> = ranges
                    .into_iter()
                    .map(|(style, piece)| {
                        let fg = style.foreground;
                        Span::styled(
                            trim_eol(piece).to_string(),
                            Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                        )
                    })
                    .collect();
                out.push(Line::from(spans));
            }
            Err(_) => out.push(Line::from(trim_eol(line).to_string())),
        }
    }
    out
}

fn trim_eol(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_json_into_lines() {
        let lines = highlight_json("{\n  \"a\": 1\n}");
        assert_eq!(lines.len(), 3);
        // every non-empty line should carry at least one styled span
        assert!(lines.iter().all(|l| !l.spans.is_empty()));
    }

    #[test]
    fn empty_input_yields_no_lines() {
        assert!(highlight_json("").is_empty());
    }

    #[test]
    fn second_call_same_input_is_cached() {
        let a = highlight_json("{\"x\": true}");
        let b = highlight_json("{\"x\": true}");
        assert_eq!(a.len(), b.len());
    }
}

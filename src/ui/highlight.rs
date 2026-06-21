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

/// Highlight `text` as JSON into ratatui lines, picking a light or dark theme.
/// When `no_color` is set, returns plain unstyled lines. Memoizes the most
/// recent (text, light, no_color) combination.
pub fn highlight_json(text: &str, light: bool, no_color: bool) -> Vec<Line<'static>> {
    let key = cache_key(text, light, no_color);
    let cache = CACHE.get_or_init(|| Mutex::new(Cache::default()));
    if !text.is_empty() {
        let guard = cache.lock().unwrap();
        if guard.key == key {
            return guard.lines.clone();
        }
    }

    let lines = if no_color {
        plain(text)
    } else {
        compute(text, light)
    };

    let mut guard = cache.lock().unwrap();
    guard.key = key;
    guard.lines = lines.clone();
    lines
}

fn cache_key(text: &str, light: bool, no_color: bool) -> String {
    format!("{}|{}|{text}", light as u8, no_color as u8)
}

fn plain(text: &str) -> Vec<Line<'static>> {
    LinesWithEndings::from(text)
        .map(|line| Line::from(trim_eol(line).to_string()))
        .collect()
}

fn compute(text: &str, light: bool) -> Vec<Line<'static>> {
    let ps = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEMES.get_or_init(ThemeSet::load_defaults);
    let syntax = ps
        .find_syntax_by_extension("json")
        .unwrap_or_else(|| ps.find_syntax_plain_text());
    let theme_name = if light {
        "InspiredGitHub"
    } else {
        "base16-ocean.dark"
    };
    let theme = &ts.themes[theme_name];
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
        let lines = highlight_json("{\n  \"a\": 1\n}", false, false);
        assert_eq!(lines.len(), 3);
        // every non-empty line should carry at least one styled span
        assert!(lines.iter().all(|l| !l.spans.is_empty()));
    }

    #[test]
    fn empty_input_yields_no_lines() {
        assert!(highlight_json("", false, false).is_empty());
    }

    #[test]
    fn second_call_same_input_is_cached() {
        let a = highlight_json("{\"x\": true}", false, false);
        let b = highlight_json("{\"x\": true}", false, false);
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn light_theme_differs_from_dark() {
        let json = "{\"k\": \"v\"}";
        let dark = highlight_json(json, false, false);
        let light = highlight_json(json, true, false);
        let dark_fg = dark[0].spans.iter().map(|s| s.style.fg).collect::<Vec<_>>();
        let light_fg = light[0]
            .spans
            .iter()
            .map(|s| s.style.fg)
            .collect::<Vec<_>>();
        // Different themes should yield at least one differing foreground color.
        assert_ne!(dark_fg, light_fg);
    }

    #[test]
    fn no_color_yields_unstyled_lines() {
        let lines = highlight_json("{\n  \"a\": 1\n}", false, true);
        assert_eq!(lines.len(), 3);
        for line in &lines {
            for span in &line.spans {
                assert_eq!(span.style.fg, None);
            }
        }
    }
}

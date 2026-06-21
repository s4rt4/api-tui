pub mod event;
pub mod input;

use crate::collection::model::{Body, Collection, Request};
use crate::http::{Response, SendOpts};
use std::collections::HashMap;
use std::path::PathBuf;
use tui_textarea::{CursorMove, TextArea};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivePanel {
    CollectionList,
    RequestEditor,
    ResponseViewer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RequestField {
    Method,
    Url,
    Query,
    Headers,
    Body,
}

pub enum StatusKind {
    Info,
    Warn,
    Error,
}

/// A pending yes/no confirmation that blocks normal input until resolved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirm {
    QuitDirty,
    DeleteRequest,
}

pub struct App {
    pub collection_path: Option<PathBuf>,
    pub collection: Collection,
    pub selected: usize,
    pub active_panel: ActivePanel,
    pub active_field: RequestField,
    pub input_mode: InputMode,
    pub response: Option<Response>,
    pub response_scroll: u16,
    pub show_response_headers: bool,
    pub status_message: Option<(StatusKind, String)>,
    pub is_loading: bool,
    pub in_flight: Option<tokio::task::JoinHandle<()>>,
    pub dirty: bool,
    pub env: String,
    pub send_opts: SendOpts,
    pub help_open: bool,
    pub confirm: Option<Confirm>,
    /// Active multi-line editor for the current field, present only in insert mode.
    pub editor: Option<TextArea<'static>>,
    pub should_quit: bool,
    /// Use a light syntax-highlighting theme instead of the default dark one.
    pub light_theme: bool,
    /// Disable colored syntax highlighting in the response viewer.
    pub no_color: bool,
}

impl App {
    pub fn new(collection: Collection, env: String, send_opts: SendOpts) -> Self {
        Self {
            collection_path: None,
            collection,
            selected: 0,
            active_panel: ActivePanel::CollectionList,
            active_field: RequestField::Url,
            input_mode: InputMode::Normal,
            response: None,
            response_scroll: 0,
            show_response_headers: false,
            status_message: None,
            is_loading: false,
            in_flight: None,
            dirty: false,
            env,
            send_opts,
            help_open: false,
            confirm: None,
            editor: None,
            should_quit: false,
            light_theme: false,
            no_color: false,
        }
    }

    pub fn select_next(&mut self) {
        let n = self.collection.requests.len();
        if n == 0 {
            return;
        }
        self.selected = (self.selected + 1) % n;
        self.response_scroll = 0;
    }

    pub fn select_prev(&mut self) {
        let n = self.collection.requests.len();
        if n == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            n - 1
        } else {
            self.selected - 1
        };
        self.response_scroll = 0;
    }

    pub fn cycle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::CollectionList => ActivePanel::RequestEditor,
            ActivePanel::RequestEditor => ActivePanel::ResponseViewer,
            ActivePanel::ResponseViewer => ActivePanel::CollectionList,
        };
    }

    pub fn select_field_next(&mut self) {
        self.active_field = match self.active_field {
            RequestField::Method => RequestField::Url,
            RequestField::Url => RequestField::Query,
            RequestField::Query => RequestField::Headers,
            RequestField::Headers => RequestField::Body,
            RequestField::Body => RequestField::Method,
        };
    }

    pub fn select_field_prev(&mut self) {
        self.active_field = match self.active_field {
            RequestField::Method => RequestField::Body,
            RequestField::Url => RequestField::Method,
            RequestField::Query => RequestField::Url,
            RequestField::Headers => RequestField::Query,
            RequestField::Body => RequestField::Headers,
        };
    }

    pub fn cycle_method(&mut self) -> bool {
        if let Some(req) = self.collection.requests.get_mut(self.selected) {
            req.method = next_method(&req.method);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn scroll_response_up(&mut self) {
        self.response_scroll = self.response_scroll.saturating_sub(1);
    }

    pub fn scroll_response_down(&mut self) {
        self.response_scroll = self.response_scroll.saturating_add(1);
    }

    /// Insert a new blank request just after the current selection (or at the
    /// front of an empty collection) and select it. Marks the collection dirty.
    pub fn add_request(&mut self) {
        let new = Request {
            name: "new-request".into(),
            method: "GET".into(),
            url: String::new(),
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };
        let idx = if self.collection.requests.is_empty() {
            0
        } else {
            self.selected + 1
        };
        self.collection.requests.insert(idx, new);
        self.selected = idx;
        self.response = None;
        self.response_scroll = 0;
        self.dirty = true;
    }

    /// Remove the selected request, clamping the selection. Returns false if the
    /// collection was already empty.
    pub fn delete_selected(&mut self) -> bool {
        if self.selected >= self.collection.requests.len() {
            return false;
        }
        self.collection.requests.remove(self.selected);
        if self.selected >= self.collection.requests.len() {
            self.selected = self.collection.requests.len().saturating_sub(1);
        }
        self.response = None;
        self.response_scroll = 0;
        self.dirty = true;
        true
    }

    /// Open the field editor for the active field, seeded with its current value.
    /// Returns false if the field is not editable in insert mode (Method, or no
    /// request selected, or not in the request panel).
    pub fn begin_edit(&mut self) -> bool {
        if self.active_panel != ActivePanel::RequestEditor {
            return false;
        }
        let field = self.active_field;
        let Some(req) = self.collection.requests.get(self.selected) else {
            return false;
        };
        let text = match field {
            RequestField::Url => req.url.clone(),
            RequestField::Query => map_to_text(&req.query, '='),
            RequestField::Headers => map_to_text(&req.headers, ':'),
            RequestField::Body => req
                .body
                .as_ref()
                .map(|b| b.content.clone())
                .unwrap_or_default(),
            RequestField::Method => return false,
        };
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(str::to_string).collect()
        };
        let mut ta = TextArea::new(lines);
        ta.move_cursor(CursorMove::Bottom);
        ta.move_cursor(CursorMove::End);
        self.editor = Some(ta);
        self.input_mode = InputMode::Insert;
        true
    }

    /// Discard the editor without applying changes.
    pub fn cancel_edit(&mut self) {
        self.editor = None;
        self.input_mode = InputMode::Normal;
    }

    /// Apply the editor's contents back to the active field, marking the
    /// collection dirty if anything changed, then leave insert mode.
    pub fn commit_edit(&mut self) {
        self.input_mode = InputMode::Normal;
        let Some(ta) = self.editor.take() else {
            return;
        };
        let field = self.active_field;
        let lines = ta.into_lines();
        let Some(req) = self.collection.requests.get_mut(self.selected) else {
            return;
        };
        let mut changed = false;
        match field {
            RequestField::Url => {
                let url = lines.join("");
                if url != req.url {
                    req.url = url;
                    changed = true;
                }
            }
            RequestField::Body => {
                let content = lines.join("\n");
                if content.trim().is_empty() {
                    if req.body.take().is_some() {
                        changed = true;
                    }
                } else if req.body.as_ref().map(|b| b.content.as_str()) != Some(content.as_str()) {
                    let kind = req
                        .body
                        .as_ref()
                        .map(|b| b.kind.clone())
                        .unwrap_or_else(|| "raw".into());
                    req.body = Some(Body { kind, content });
                    changed = true;
                }
            }
            RequestField::Query => {
                let map = parse_map(&lines, '=');
                if map != req.query {
                    req.query = map;
                    changed = true;
                }
            }
            RequestField::Headers => {
                let map = parse_map(&lines, ':');
                if map != req.headers {
                    req.headers = map;
                    changed = true;
                }
            }
            RequestField::Method => {}
        }
        if changed {
            self.dirty = true;
        }
    }

    /// Apply externally-edited body content (from `$EDITOR`). Returns true if it
    /// changed the request.
    pub fn set_body_content(&mut self, content: String) -> bool {
        let Some(req) = self.collection.requests.get_mut(self.selected) else {
            return false;
        };
        if content.trim().is_empty() {
            return req.body.take().is_some();
        }
        if req.body.as_ref().map(|b| b.content.as_str()) == Some(content.as_str()) {
            return false;
        }
        let kind = req
            .body
            .as_ref()
            .map(|b| b.kind.clone())
            .unwrap_or_else(|| "raw".into());
        req.body = Some(Body { kind, content });
        true
    }

    /// Current body text of the selected request, for handing off to `$EDITOR`.
    pub fn current_body(&self) -> String {
        self.collection
            .requests
            .get(self.selected)
            .and_then(|r| r.body.as_ref())
            .map(|b| b.content.clone())
            .unwrap_or_default()
    }
}

/// Render a key/value map as one `key<sep> value` line per entry, sorted by key.
fn map_to_text(map: &HashMap<String, String>, sep: char) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
        .iter()
        .map(|(k, v)| {
            if sep == ':' {
                format!("{}: {}", k, v)
            } else {
                format!("{}{}{}", k, sep, v)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse `key<sep>value` lines back into a map. Blank lines and lines without
/// the separator are skipped; keys/values are trimmed. Splits on the first
/// separator so values may contain it.
fn parse_map(lines: &[String], sep: char) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(sep) {
            let k = k.trim();
            if !k.is_empty() {
                map.insert(k.to_string(), v.trim().to_string());
            }
        }
    }
    map
}

fn next_method(current: &str) -> String {
    match current.to_ascii_uppercase().as_str() {
        "GET" => "POST".into(),
        "POST" => "PUT".into(),
        "PUT" => "PATCH".into(),
        "PATCH" => "DELETE".into(),
        "DELETE" => "GET".into(),
        _ => "GET".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::model::Request;

    fn fixture() -> App {
        let mk = |name: &str, method: &str, url: &str| Request {
            name: name.into(),
            method: method.into(),
            url: url.into(),
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };
        let c = Collection {
            requests: vec![
                mk("a", "GET", "/a"),
                mk("b", "POST", "/b"),
                mk("c", "GET", "/c"),
            ],
            ..Default::default()
        };
        App::new(c, "default".into(), SendOpts::default())
    }

    #[test]
    fn select_next_wraps() {
        let mut app = fixture();
        app.selected = 2;
        app.select_next();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn select_prev_wraps() {
        let mut app = fixture();
        app.selected = 0;
        app.select_prev();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn cycle_panel_full_loop() {
        let mut app = fixture();
        assert_eq!(app.active_panel, ActivePanel::CollectionList);
        app.cycle_panel();
        assert_eq!(app.active_panel, ActivePanel::RequestEditor);
        app.cycle_panel();
        assert_eq!(app.active_panel, ActivePanel::ResponseViewer);
        app.cycle_panel();
        assert_eq!(app.active_panel, ActivePanel::CollectionList);
    }

    #[test]
    fn select_on_empty_is_noop() {
        let mut app = App::new(Collection::default(), "default".into(), SendOpts::default());
        app.select_next();
        app.select_prev();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn select_field_cycles() {
        let mut app = fixture();
        assert_eq!(app.active_field, RequestField::Url);
        app.select_field_next();
        assert_eq!(app.active_field, RequestField::Query);
        app.select_field_prev();
        assert_eq!(app.active_field, RequestField::Url);
        app.select_field_prev();
        assert_eq!(app.active_field, RequestField::Method);
    }

    #[test]
    fn cycle_method_advances_and_marks_dirty() {
        let mut app = fixture();
        assert!(!app.dirty);
        assert_eq!(app.collection.requests[0].method, "GET");
        assert!(app.cycle_method());
        assert_eq!(app.collection.requests[0].method, "POST");
        assert!(app.dirty);
    }

    #[test]
    fn cycle_method_full_loop() {
        let mut app = fixture();
        for expected in &["POST", "PUT", "PATCH", "DELETE", "GET"] {
            app.cycle_method();
            assert_eq!(app.collection.requests[0].method, *expected);
        }
    }

    #[test]
    fn begin_edit_method_returns_false() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Method;
        assert!(!app.begin_edit());
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn begin_edit_outside_request_panel_returns_false() {
        let mut app = fixture();
        app.active_panel = ActivePanel::CollectionList;
        app.active_field = RequestField::Url;
        assert!(!app.begin_edit());
    }

    #[test]
    fn commit_edit_url_appends_and_dirties() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Url;
        assert!(app.begin_edit());
        app.editor.as_mut().unwrap().insert_str("X");
        app.commit_edit();
        assert_eq!(app.collection.requests[0].url, "/aX");
        assert!(app.dirty);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.editor.is_none());
    }

    #[test]
    fn commit_edit_unchanged_stays_clean() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Url;
        app.begin_edit();
        app.commit_edit();
        assert!(!app.dirty);
    }

    #[test]
    fn cancel_edit_discards() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Url;
        app.begin_edit();
        app.editor.as_mut().unwrap().insert_str("ZZZ");
        app.cancel_edit();
        assert_eq!(app.collection.requests[0].url, "/a");
        assert!(!app.dirty);
        assert!(app.editor.is_none());
    }

    #[test]
    fn commit_edit_body_sets_content() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Body;
        app.begin_edit();
        app.editor.as_mut().unwrap().insert_str("{\"a\":1}");
        app.commit_edit();
        let body = app.collection.requests[0].body.as_ref().unwrap();
        assert_eq!(body.content, "{\"a\":1}");
        assert_eq!(body.kind, "raw");
        assert!(app.dirty);
    }

    #[test]
    fn commit_empty_body_clears_to_none() {
        let mut app = fixture();
        app.collection.requests[0].body = Some(Body {
            kind: "raw".into(),
            content: "x".into(),
        });
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Body;
        app.editor = Some(TextArea::default());
        app.input_mode = InputMode::Insert;
        app.commit_edit();
        assert!(app.collection.requests[0].body.is_none());
        assert!(app.dirty);
    }

    #[test]
    fn commit_edit_headers_parses_lines() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Headers;
        app.begin_edit();
        app.editor
            .as_mut()
            .unwrap()
            .insert_str("Authorization: Bearer xyz");
        app.commit_edit();
        assert_eq!(
            app.collection.requests[0]
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer xyz")
        );
        assert!(app.dirty);
    }

    #[test]
    fn commit_edit_query_parses_lines() {
        let mut app = fixture();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Query;
        app.begin_edit();
        app.editor.as_mut().unwrap().insert_str("page=2");
        app.commit_edit();
        assert_eq!(
            app.collection.requests[0]
                .query
                .get("page")
                .map(String::as_str),
            Some("2")
        );
    }

    #[test]
    fn map_text_round_trips() {
        let mut m = HashMap::new();
        m.insert("A".to_string(), "1".to_string());
        m.insert("B".to_string(), "two words".to_string());
        let text = map_to_text(&m, ':');
        assert_eq!(text, "A: 1\nB: two words");
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        assert_eq!(parse_map(&lines, ':'), m);
    }

    #[test]
    fn parse_map_skips_blank_and_separatorless_lines() {
        let lines = vec![
            "x=1".to_string(),
            String::new(),
            "garbage".to_string(),
            "y=2".to_string(),
        ];
        let m = parse_map(&lines, '=');
        assert_eq!(m.len(), 2);
        assert_eq!(m.get("x").map(String::as_str), Some("1"));
        assert_eq!(m.get("y").map(String::as_str), Some("2"));
    }

    #[test]
    fn set_body_content_applies_and_clears() {
        let mut app = fixture();
        assert!(app.set_body_content("hi".into()));
        assert_eq!(
            app.collection.requests[0].body.as_ref().unwrap().content,
            "hi"
        );
        assert!(!app.set_body_content("hi".into())); // unchanged
        assert!(app.set_body_content("  ".into())); // blank clears
        assert!(app.collection.requests[0].body.is_none());
    }

    #[test]
    fn add_request_inserts_after_selection_and_selects_it() {
        let mut app = fixture();
        app.selected = 0;
        app.add_request();
        assert_eq!(app.collection.requests.len(), 4);
        assert_eq!(app.selected, 1);
        assert_eq!(app.collection.requests[1].name, "new-request");
        assert!(app.dirty);
    }

    #[test]
    fn add_request_on_empty_inserts_at_front() {
        let mut app = App::new(Collection::default(), "default".into(), SendOpts::default());
        app.add_request();
        assert_eq!(app.collection.requests.len(), 1);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn delete_selected_removes_and_clamps() {
        let mut app = fixture();
        app.selected = 2;
        assert!(app.delete_selected());
        assert_eq!(app.collection.requests.len(), 2);
        assert_eq!(app.selected, 1);
        assert!(app.dirty);
    }

    #[test]
    fn delete_selected_middle_keeps_index() {
        let mut app = fixture();
        app.selected = 1;
        app.delete_selected();
        assert_eq!(app.collection.requests.len(), 2);
        assert_eq!(app.selected, 1);
        assert_eq!(app.collection.requests[1].name, "c");
    }

    #[test]
    fn delete_selected_on_empty_is_false() {
        let mut app = App::new(Collection::default(), "default".into(), SendOpts::default());
        assert!(!app.delete_selected());
    }

    #[test]
    fn scroll_saturates_at_zero() {
        let mut app = fixture();
        app.scroll_response_up();
        assert_eq!(app.response_scroll, 0);
    }

    #[test]
    fn scroll_down_increments() {
        let mut app = fixture();
        app.scroll_response_down();
        app.scroll_response_down();
        assert_eq!(app.response_scroll, 2);
    }

    #[test]
    fn select_resets_scroll() {
        let mut app = fixture();
        app.response_scroll = 10;
        app.select_next();
        assert_eq!(app.response_scroll, 0);
    }
}

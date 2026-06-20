use crate::app::{ActivePanel, App, Confirm, InputMode, RequestField};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    Send,
    Cancel,
    Save,
    SaveAndQuit,
    EditExternal,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    if let Some(confirm) = app.confirm {
        return handle_confirm(app, confirm, key);
    }

    if app.help_open {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
        ) {
            app.help_open = false;
        }
        return None;
    }

    match app.input_mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::Insert => {
            handle_insert(app, key);
            None
        }
    }
}

/// While a confirmation is pending, only the dialog's own keys are meaningful;
/// everything else is swallowed so the user can't act on the backdrop.
fn handle_confirm(app: &mut App, confirm: Confirm, key: KeyEvent) -> Option<Action> {
    let cancel = matches!(
        key.code,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc
    );
    if cancel {
        app.confirm = None;
        return None;
    }
    match confirm {
        Confirm::QuitDirty => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm = None;
                app.should_quit = true;
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                app.confirm = None;
                return Some(Action::SaveAndQuit);
            }
            _ => {}
        },
        Confirm::DeleteRequest => {
            if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                app.confirm = None;
                app.delete_selected();
            }
        }
    }
    None
}

fn request_quit(app: &mut App) {
    if app.dirty {
        app.confirm = Some(Confirm::QuitDirty);
    } else {
        app.should_quit = true;
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        request_quit(app);
        return None;
    }

    match key.code {
        KeyCode::Char('q') => {
            request_quit(app);
            None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            match app.active_panel {
                ActivePanel::CollectionList => app.select_prev(),
                ActivePanel::RequestEditor => app.select_field_prev(),
                ActivePanel::ResponseViewer => app.scroll_response_up(),
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            match app.active_panel {
                ActivePanel::CollectionList => app.select_next(),
                ActivePanel::RequestEditor => app.select_field_next(),
                ActivePanel::ResponseViewer => app.scroll_response_down(),
            }
            None
        }
        KeyCode::Tab => {
            app.cycle_panel();
            None
        }
        KeyCode::Char('?') => {
            app.help_open = true;
            None
        }
        KeyCode::Char('s') => {
            if app.is_loading {
                None
            } else {
                Some(Action::Send)
            }
        }
        KeyCode::Char('h') => {
            app.show_response_headers = !app.show_response_headers;
            None
        }
        KeyCode::Char('m') => {
            app.cycle_method();
            None
        }
        KeyCode::Char('w') => Some(Action::Save),
        KeyCode::Char('a') => {
            app.add_request();
            None
        }
        KeyCode::Char('d') => {
            if !app.collection.requests.is_empty() {
                app.confirm = Some(Confirm::DeleteRequest);
            }
            None
        }
        KeyCode::Char('e') => {
            app.begin_edit();
            None
        }
        KeyCode::Char('E') => {
            if app.active_panel == ActivePanel::RequestEditor
                && app.active_field == RequestField::Body
                && !app.collection.requests.is_empty()
            {
                Some(Action::EditExternal)
            } else {
                None
            }
        }
        KeyCode::Esc => {
            if app.is_loading {
                Some(Action::Cancel)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn handle_insert(app: &mut App, key: KeyEvent) {
    // Esc commits for every field. Enter commits single-line URL but inserts a
    // newline in the multi-line fields (body, headers, query).
    match key.code {
        KeyCode::Esc => {
            app.commit_edit();
            return;
        }
        KeyCode::Enter if app.active_field == RequestField::Url => {
            app.commit_edit();
            return;
        }
        _ => {}
    }

    if let Some(editor) = app.editor.as_mut() {
        let _ = editor.input(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::model::{Collection, Request};
    use crate::http::SendOpts;
    use crossterm::event::KeyEventKind;
    use std::collections::HashMap;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn empty_app() -> App {
        App::new(Collection::default(), "default".into(), SendOpts::default())
    }

    fn app_with_request() -> App {
        let mut c = Collection::default();
        c.requests.push(Request {
            name: "x".into(),
            method: "GET".into(),
            url: "/api".into(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: None,
        });
        App::new(c, "default".into(), SendOpts::default())
    }

    #[test]
    fn q_sets_quit() {
        let mut app = empty_app();
        assert!(handle_key(&mut app, key(KeyCode::Char('q'))).is_none());
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_sets_quit() {
        let mut app = empty_app();
        handle_key(&mut app, ctrl(KeyCode::Char('c')));
        assert!(app.should_quit);
    }

    #[test]
    fn tab_cycles_panel() {
        let mut app = empty_app();
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.active_panel, ActivePanel::RequestEditor);
    }

    #[test]
    fn help_toggle() {
        let mut app = empty_app();
        handle_key(&mut app, key(KeyCode::Char('?')));
        assert!(app.help_open);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.help_open);
    }

    #[test]
    fn quit_blocked_when_help_open() {
        let mut app = empty_app();
        app.help_open = true;
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert!(!app.help_open);
    }

    #[test]
    fn s_returns_send_action() {
        let mut app = empty_app();
        let action = handle_key(&mut app, key(KeyCode::Char('s')));
        assert!(matches!(action, Some(Action::Send)));
    }

    #[test]
    fn s_blocked_while_loading() {
        let mut app = empty_app();
        app.is_loading = true;
        let action = handle_key(&mut app, key(KeyCode::Char('s')));
        assert!(action.is_none());
    }

    #[test]
    fn esc_cancels_when_loading() {
        let mut app = empty_app();
        app.is_loading = true;
        let action = handle_key(&mut app, key(KeyCode::Esc));
        assert!(matches!(action, Some(Action::Cancel)));
    }

    #[test]
    fn esc_noop_when_idle() {
        let mut app = empty_app();
        let action = handle_key(&mut app, key(KeyCode::Esc));
        assert!(action.is_none());
    }

    #[test]
    fn h_toggles_response_headers() {
        let mut app = empty_app();
        assert!(!app.show_response_headers);
        handle_key(&mut app, key(KeyCode::Char('h')));
        assert!(app.show_response_headers);
        handle_key(&mut app, key(KeyCode::Char('h')));
        assert!(!app.show_response_headers);
    }

    #[test]
    fn arrow_in_response_panel_scrolls_not_select() {
        let mut app = empty_app();
        app.active_panel = ActivePanel::ResponseViewer;
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.response_scroll, 1);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn arrow_in_request_editor_cycles_field() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        assert_eq!(app.active_field, RequestField::Url);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.active_field, RequestField::Query);
    }

    #[test]
    fn m_cycles_method_and_marks_dirty() {
        let mut app = app_with_request();
        handle_key(&mut app, key(KeyCode::Char('m')));
        assert_eq!(app.collection.requests[0].method, "POST");
        assert!(app.dirty);
    }

    #[test]
    fn w_returns_save_action() {
        let mut app = empty_app();
        let action = handle_key(&mut app, key(KeyCode::Char('w')));
        assert!(matches!(action, Some(Action::Save)));
    }

    fn start_url_edit() -> App {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Url;
        handle_key(&mut app, key(KeyCode::Char('e')));
        app
    }

    #[test]
    fn e_enters_insert_for_url() {
        let app = start_url_edit();
        assert_eq!(app.input_mode, InputMode::Insert);
        assert!(app.editor.is_some());
    }

    #[test]
    fn e_blocked_when_not_in_request_editor() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::CollectionList;
        handle_key(&mut app, key(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.editor.is_none());
    }

    #[test]
    fn e_edits_body_field() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Body;
        handle_key(&mut app, key(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Insert);
        assert!(app.editor.is_some());
    }

    #[test]
    fn insert_mode_chars_commit_to_url() {
        let mut app = start_url_edit();
        handle_key(&mut app, key(KeyCode::Char('z')));
        handle_key(&mut app, key(KeyCode::Enter)); // Enter commits single-line URL
        assert_eq!(app.collection.requests[0].url, "/apiz");
        assert!(app.dirty);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn insert_mode_backspace_removes() {
        let mut app = start_url_edit();
        handle_key(&mut app, key(KeyCode::Backspace));
        handle_key(&mut app, key(KeyCode::Esc)); // Esc commits
        assert_eq!(app.collection.requests[0].url, "/ap");
    }

    #[test]
    fn enter_inserts_newline_in_body() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Body;
        handle_key(&mut app, key(KeyCode::Char('e')));
        handle_key(&mut app, key(KeyCode::Char('a')));
        handle_key(&mut app, key(KeyCode::Enter)); // newline, not commit
        assert_eq!(app.input_mode, InputMode::Insert); // still editing
        handle_key(&mut app, key(KeyCode::Char('b')));
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.collection.requests[0].body.as_ref().unwrap().content, "a\nb");
    }

    #[test]
    fn esc_in_insert_mode_returns_to_normal() {
        let mut app = start_url_edit();
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.editor.is_none());
    }

    #[test]
    fn enter_in_url_insert_returns_to_normal() {
        let mut app = start_url_edit();
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn shift_e_returns_edit_external_on_body() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Body;
        let action = handle_key(&mut app, key(KeyCode::Char('E')));
        assert!(matches!(action, Some(Action::EditExternal)));
    }

    #[test]
    fn shift_e_noop_off_body() {
        let mut app = app_with_request();
        app.active_panel = ActivePanel::RequestEditor;
        app.active_field = RequestField::Url;
        let action = handle_key(&mut app, key(KeyCode::Char('E')));
        assert!(action.is_none());
    }

    #[test]
    fn confirm_quit_w_returns_save_and_quit() {
        let mut app = app_with_request();
        app.dirty = true;
        handle_key(&mut app, key(KeyCode::Char('q')));
        let action = handle_key(&mut app, key(KeyCode::Char('w')));
        assert!(matches!(action, Some(Action::SaveAndQuit)));
        assert!(app.confirm.is_none());
    }

    #[test]
    fn q_when_dirty_opens_confirm_not_quit() {
        let mut app = app_with_request();
        app.dirty = true;
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert_eq!(app.confirm, Some(Confirm::QuitDirty));
    }

    #[test]
    fn confirm_quit_y_quits() {
        let mut app = app_with_request();
        app.dirty = true;
        handle_key(&mut app, key(KeyCode::Char('q')));
        handle_key(&mut app, key(KeyCode::Char('y')));
        assert!(app.should_quit);
        assert!(app.confirm.is_none());
    }

    #[test]
    fn confirm_quit_n_cancels() {
        let mut app = app_with_request();
        app.dirty = true;
        handle_key(&mut app, key(KeyCode::Char('q')));
        handle_key(&mut app, key(KeyCode::Char('n')));
        assert!(!app.should_quit);
        assert!(app.confirm.is_none());
    }

    #[test]
    fn ctrl_c_when_dirty_confirms() {
        let mut app = app_with_request();
        app.dirty = true;
        handle_key(&mut app, ctrl(KeyCode::Char('c')));
        assert!(!app.should_quit);
        assert_eq!(app.confirm, Some(Confirm::QuitDirty));
    }

    #[test]
    fn a_adds_request() {
        let mut app = empty_app();
        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.collection.requests.len(), 1);
        assert!(app.dirty);
    }

    #[test]
    fn d_opens_delete_confirm_when_requests_exist() {
        let mut app = app_with_request();
        handle_key(&mut app, key(KeyCode::Char('d')));
        assert_eq!(app.confirm, Some(Confirm::DeleteRequest));
    }

    #[test]
    fn d_noop_on_empty_collection() {
        let mut app = empty_app();
        handle_key(&mut app, key(KeyCode::Char('d')));
        assert!(app.confirm.is_none());
    }

    #[test]
    fn confirm_delete_y_removes_request() {
        let mut app = app_with_request();
        handle_key(&mut app, key(KeyCode::Char('d')));
        handle_key(&mut app, key(KeyCode::Char('y')));
        assert!(app.collection.requests.is_empty());
        assert!(app.confirm.is_none());
    }

    #[test]
    fn confirm_swallows_other_keys() {
        let mut app = app_with_request();
        app.confirm = Some(Confirm::DeleteRequest);
        handle_key(&mut app, key(KeyCode::Char('s')));
        assert_eq!(app.confirm, Some(Confirm::DeleteRequest));
        assert_eq!(app.collection.requests.len(), 1);
    }

    #[test]
    fn q_in_insert_mode_does_not_quit() {
        let mut app = start_url_edit();
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        handle_key(&mut app, key(KeyCode::Esc)); // commit
        assert_eq!(app.collection.requests[0].url, "/apiq");
    }
}

use crate::app::{
    event::AppEvent,
    input::{self, Action},
    App, StatusKind,
};
use crate::collection::{self, build, model::Collection};
use crate::config::Cli;
use crate::error::ApiTesterError;
use crate::http::{self, Response, SendOpts, StatusClass};
use crate::tui::Tui;
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};

pub async fn run_tui(cli: Cli) -> Result<()> {
    let collection = match &cli.collection {
        Some(p) => collection::load(p)?,
        None => Collection::default(),
    };
    let send_opts = SendOpts {
        timeout: cli.timeout_duration(),
        insecure: cli.insecure,
        follow_redirects: !cli.no_redirect,
        proxy: cli.proxy.clone(),
    };
    let mut app = App::new(collection, cli.env.clone(), send_opts);
    app.collection_path = cli.collection.clone();
    app.light_theme = cli.theme.is_light();
    app.no_color = cli.no_color;
    if cli.insecure {
        app.status_message = Some((
            StatusKind::Warn,
            "⚠ TLS verification disabled (--insecure)".into(),
        ));
    }

    let mut tui = Tui::new()?;
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    spawn_event_task(tx.clone());
    spawn_tick_task(tx.clone());

    tui.terminal.draw(|f| ui::render(f, &app))?;

    while let Some(ev) = rx.recv().await {
        match ev {
            AppEvent::Key(k) => {
                let action = input::handle_key(&mut app, k);
                if app.should_quit {
                    break;
                }
                if let Some(a) = action {
                    match a {
                        Action::Send => spawn_send(&mut app, &tx),
                        Action::Cancel => cancel_inflight(&mut app),
                        Action::Save => save_collection(&mut app),
                        Action::SaveAndQuit => {
                            save_collection(&mut app);
                            if !app.dirty {
                                app.should_quit = true;
                            }
                        }
                        Action::EditExternal => external_edit(&mut app, &mut tui)?,
                        Action::Export => export_response(&mut app),
                        Action::Yank => yank_response(&mut app),
                    }
                }
                if app.should_quit {
                    break;
                }
            }
            AppEvent::RequestDone(result) => handle_request_done(&mut app, result),
            AppEvent::RequestStarted | AppEvent::Tick => {}
            AppEvent::Quit => break,
        }
        tui.terminal.draw(|f| ui::render(f, &app))?;
    }

    Ok(())
}

fn spawn_event_task(tx: UnboundedSender<AppEvent>) {
    tokio::task::spawn_blocking(move || loop {
        match event::poll(Duration::from_millis(250)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                    if tx.send(AppEvent::Key(k)).is_err() {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(_) => break,
            },
            Ok(false) => continue,
            Err(_) => break,
        }
    });
}

fn spawn_tick_task(tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
}

fn spawn_send(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let req = match app.collection.requests.get(app.selected) {
        Some(r) => r.clone(),
        None => {
            app.status_message = Some((StatusKind::Warn, "no request selected".into()));
            return;
        }
    };

    let env_vars = build::resolve_env(&app.collection, &app.env);
    let built = match build::build_effective(&req, app.collection.base_url.as_deref(), &env_vars) {
        Ok(b) => b,
        Err(e) => {
            app.status_message = Some((StatusKind::Error, e.to_string()));
            return;
        }
    };

    app.is_loading = true;
    app.response = None;
    app.response_scroll = 0;
    app.status_message = Some((
        StatusKind::Info,
        format!("→ {} {}", built.method, built.url),
    ));

    let opts = app.send_opts.clone();
    let send_tx = tx.clone();
    let handle = tokio::spawn(async move {
        let result = http::send(
            &built.method,
            &built.url,
            &built.query,
            &built.headers,
            built.body.as_deref(),
            &opts,
        )
        .await;
        let _ = send_tx.send(AppEvent::RequestDone(result));
    });
    app.in_flight = Some(handle);
}

fn cancel_inflight(app: &mut App) {
    if let Some(handle) = app.in_flight.take() {
        handle.abort();
        app.is_loading = false;
        app.status_message = Some((StatusKind::Warn, "request cancelled".into()));
    }
}

fn save_collection(app: &mut App) {
    let Some(path) = app.collection_path.clone() else {
        app.status_message = Some((
            StatusKind::Warn,
            "no path — pass a .toml as arg to enable save".into(),
        ));
        return;
    };
    match collection::save(&path, &app.collection) {
        Ok(()) => {
            app.dirty = false;
            app.status_message = Some((StatusKind::Info, format!("saved {}", path.display())));
        }
        Err(e) => {
            app.status_message = Some((StatusKind::Error, format!("save failed: {}", e)));
        }
    }
}

/// Suspend the TUI, open the selected request's body in `$EDITOR`, then resume
/// and apply whatever was saved.
fn external_edit(app: &mut App, tui: &mut Tui) -> Result<()> {
    if app.collection.requests.get(app.selected).is_none() {
        return Ok(());
    }
    let current = app.current_body();

    tui.suspend()?;
    let result = run_editor(&current);
    tui.resume()?;

    match result {
        Ok(Some(new_content)) => {
            if app.set_body_content(new_content) {
                app.dirty = true;
            }
            app.status_message = Some((StatusKind::Info, "body updated from $EDITOR".into()));
        }
        Ok(None) => {
            app.status_message = Some((StatusKind::Warn, "editor exited — body unchanged".into()));
        }
        Err(e) => {
            app.status_message = Some((StatusKind::Error, format!("editor failed: {}", e)));
        }
    }
    Ok(())
}

/// Write `content` to a temp file, launch `$VISUAL`/`$EDITOR` (notepad on
/// Windows / vi elsewhere as fallback) on it, and return the saved content.
/// Returns `Ok(None)` if the editor exited non-zero.
fn run_editor(content: &str) -> std::io::Result<Option<String>> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    let mut path = std::env::temp_dir();
    path.push(format!("apitester-body-{}.json", std::process::id()));
    std::fs::write(&path, content)?;

    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("notepad");
    let args: Vec<&str> = parts.collect();
    let status = std::process::Command::new(program)
        .args(&args)
        .arg(&path)
        .status()?;

    let result = if status.success() {
        Some(std::fs::read_to_string(&path)?)
    } else {
        None
    };
    let _ = std::fs::remove_file(&path);
    Ok(result)
}

/// Write the current response body (pretty-printed) to `<request-name>.json|txt`
/// in the working directory.
fn export_response(app: &mut App) {
    let Some(resp) = &app.response else {
        return;
    };
    let name = app
        .collection
        .requests
        .get(app.selected)
        .map(|r| r.name.as_str())
        .unwrap_or("response");
    let ext = if resp.is_json() { "json" } else { "txt" };
    let filename = format!("{}.{}", sanitize_filename(name), ext);
    match std::fs::write(&filename, resp.pretty_body()) {
        Ok(()) => {
            app.status_message = Some((StatusKind::Info, format!("exported → {}", filename)));
        }
        Err(e) => {
            app.status_message = Some((StatusKind::Error, format!("export failed: {}", e)));
        }
    }
}

/// Copy the current response body to the system clipboard.
fn yank_response(app: &mut App) {
    let Some(resp) = &app.response else {
        return;
    };
    let body = resp.pretty_body();
    let result = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(body));
    match result {
        Ok(()) => {
            app.status_message = Some((StatusKind::Info, "response copied to clipboard".into()));
        }
        Err(e) => {
            app.status_message = Some((StatusKind::Error, format!("clipboard failed: {}", e)));
        }
    }
}

/// Reduce an arbitrary request name to a safe filename stem.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "response".to_string()
    } else {
        trimmed.to_string()
    }
}

fn handle_request_done(app: &mut App, result: Result<Response, ApiTesterError>) {
    app.is_loading = false;
    app.in_flight = None;
    match result {
        Ok(resp) => {
            let kind = match resp.status_class() {
                StatusClass::ClientError | StatusClass::ServerError => StatusKind::Warn,
                _ => StatusKind::Info,
            };
            app.status_message = Some((
                kind,
                format!("← {} in {}ms", resp.status, resp.elapsed.as_millis()),
            ));
            app.response = Some(resp);
        }
        Err(e) => {
            app.status_message = Some((StatusKind::Error, format!("✗ {}", e)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_filename;

    #[test]
    fn sanitize_replaces_unsafe_chars() {
        assert_eq!(sanitize_filename("Get Status"), "Get_Status");
        assert_eq!(sanitize_filename("user/by:id?x"), "user_by_id_x");
    }

    #[test]
    fn sanitize_keeps_safe_chars() {
        assert_eq!(sanitize_filename("get-user_01"), "get-user_01");
    }

    #[test]
    fn sanitize_empty_falls_back() {
        assert_eq!(sanitize_filename("///"), "response");
    }
}

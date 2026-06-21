use crate::app::{App, InputMode, RequestField, StatusKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let line = match &app.status_message {
        Some((kind, msg)) => Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(status_color(kind)),
        )),
        None => Line::from(""),
    };
    frame.render_widget(Paragraph::new(line), area);
}

pub fn render_keys(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(query) = &app.search_input {
        let line = Line::from(vec![
            Span::styled(
                " SEARCH ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  /"),
            Span::raw(query.clone()),
            Span::styled("▏", Style::default().fg(Color::Yellow)),
            Span::styled(
                "   [Enter] find  [Esc] cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    if let Some(search) = &app.search {
        let summary = if search.matches.is_empty() {
            format!("no matches for \"{}\"", search.query)
        } else {
            format!(
                "match {}/{} for \"{}\"   [n] next  [N] prev  [/] new",
                search.current + 1,
                search.matches.len(),
                search.query
            )
        };
        let color = if search.matches.is_empty() {
            Color::Yellow
        } else {
            Color::Cyan
        };
        let line = Line::from(Span::styled(summary, Style::default().fg(color)));
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let line = if app.input_mode == InputMode::Insert {
        let hint = match app.active_field {
            RequestField::Url => "editing URL  [Enter/Esc] save",
            RequestField::Body => "editing body  [Esc] save  [Enter] newline",
            RequestField::Headers => "editing headers (Key: value per line)  [Esc] save",
            RequestField::Query => "editing query (key=value per line)  [Esc] save",
            RequestField::Method => "[Esc] exit",
        };
        Line::from(vec![
            Span::styled(
                " INSERT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(hint, Style::default().fg(Color::DarkGray)),
        ])
    } else if app.is_loading {
        styled_hints("[Esc] cancel  [Tab] panel  [?] help  [q] quit")
    } else {
        styled_hints("[↑↓/jk] nav  [s] send  [e] edit  [E] $EDITOR  [m] method  [a] add  [d] del  [w] save  [o] export  [y] yank  [Tab] panel  [h] hdrs  [/] find  [H] history  [?] help  [q] quit")
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn styled_hints(s: &str) -> Line<'static> {
    Line::from(Span::styled(
        s.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    ))
}

fn status_color(kind: &StatusKind) -> Color {
    match kind {
        StatusKind::Info => Color::Cyan,
        StatusKind::Warn => Color::Yellow,
        StatusKind::Error => Color::Red,
    }
}

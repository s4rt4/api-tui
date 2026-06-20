use crate::app::{ActivePanel, App, InputMode, RequestField};
use crate::ui::style::{method_color, panel_border};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if app.input_mode == InputMode::Insert && app.editor.is_some() {
        render_editor(frame, area, app);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" request ")
        .border_style(panel_border(app, ActivePanel::RequestEditor));

    let panel_active = app.active_panel == ActivePanel::RequestEditor;
    let in_insert = app.input_mode == InputMode::Insert;

    let lines: Vec<Line> = if let Some(req) = app.collection.requests.get(app.selected) {
        let mut out = vec![
            Line::from(vec![
                label(
                    "Method  ",
                    panel_active && app.active_field == RequestField::Method,
                ),
                Span::styled(
                    req.method.to_ascii_uppercase(),
                    Style::default().fg(method_color(&req.method)),
                ),
                Span::styled(
                    "   [m] cycle",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                ),
            ]),
            url_line(
                req.url.as_str(),
                panel_active && app.active_field == RequestField::Url,
                in_insert && app.active_field == RequestField::Url,
            ),
        ];

        if !req.query.is_empty() {
            let mut entries: Vec<_> = req.query.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let qs = entries
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            out.push(Line::from(vec![
                label(
                    "Query   ",
                    panel_active && app.active_field == RequestField::Query,
                ),
                Span::raw(qs),
            ]));
        } else {
            out.push(Line::from(vec![
                label(
                    "Query   ",
                    panel_active && app.active_field == RequestField::Query,
                ),
                empty_marker(),
            ]));
        }

        if !req.headers.is_empty() {
            out.push(Line::from(label(
                "Headers ",
                panel_active && app.active_field == RequestField::Headers,
            )));
            let mut entries: Vec<_> = req.headers.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (k, v) in entries {
                out.push(Line::from(format!("  {}: {}", k, v)));
            }
        } else {
            out.push(Line::from(vec![
                label(
                    "Headers ",
                    panel_active && app.active_field == RequestField::Headers,
                ),
                empty_marker(),
            ]));
        }

        if let Some(body) = &req.body {
            out.push(Line::from(vec![
                label(
                    "Body    ",
                    panel_active && app.active_field == RequestField::Body,
                ),
                Span::styled(
                    format!("[{}]", body.kind),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            for body_line in body.content.lines().take(20) {
                out.push(Line::from(format!("  {}", body_line)));
            }
        } else {
            out.push(Line::from(vec![
                label(
                    "Body    ",
                    panel_active && app.active_field == RequestField::Body,
                ),
                empty_marker(),
            ]));
        }
        out
    } else {
        vec![Line::from(Span::styled(
            "No request — press [a] to add one, or load a .toml file",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_editor(frame: &mut Frame, area: Rect, app: &App) {
    let (field, hint) = match app.active_field {
        RequestField::Url => ("URL", "Enter/Esc save"),
        RequestField::Body => ("body", "Esc save · Enter newline"),
        RequestField::Headers => ("headers", "one \"Key: value\" per line · Esc save"),
        RequestField::Query => ("query", "one \"key=value\" per line · Esc save"),
        RequestField::Method => ("method", ""),
    };
    let mut editor = app.editor.as_ref().unwrap().clone();
    editor.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" edit {} — {} ", field, hint))
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(&editor, area);
}

fn label(s: &str, active: bool) -> Span<'static> {
    let style = if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Span::styled(format!("{}: ", s), style)
}

fn empty_marker() -> Span<'static> {
    Span::styled(
        "<empty>",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    )
}

fn url_line(url: &str, active: bool, insert: bool) -> Line<'static> {
    let mut spans = vec![label("URL     ", active)];
    spans.push(Span::raw(url.to_string()));
    if insert {
        spans.push(Span::styled(
            "█",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    }
    Line::from(spans)
}

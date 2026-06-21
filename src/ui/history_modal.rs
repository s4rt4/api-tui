//! Modal listing recent request history (newest first), loaded from the
//! persistent JSONL store when opened with `H`.

use crate::history::{format_ts, HistoryEntry};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, entries: &[HistoryEntry]) {
    let area = centered(80, 70, frame.area());

    let mut lines = vec![
        Line::from(Span::styled(
            "Request history (newest first)",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no history yet — send a request first",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for e in entries {
            lines.push(entry_line(e));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc, H, or q to close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" history ")
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn entry_line(e: &HistoryEntry) -> Line<'static> {
    let outcome: Span<'static> = match (e.status, &e.error) {
        (Some(status), _) => Span::styled(
            format!("{status:<3}"),
            Style::default().fg(status_color(status)),
        ),
        (None, Some(_)) => Span::styled("ERR".to_string(), Style::default().fg(Color::Red)),
        (None, None) => Span::raw("???".to_string()),
    };

    let timing = match (e.elapsed_ms, &e.error) {
        (Some(ms), _) => format!(" {ms}ms"),
        (None, Some(err)) => format!(" {err}"),
        (None, None) => String::new(),
    };

    Line::from(vec![
        Span::styled(
            format!("{} ", format_ts(e.ts_ms)),
            Style::default().fg(Color::DarkGray),
        ),
        outcome,
        Span::styled(
            format!(" {:<6}", e.method),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(format!("{} ", e.name)),
        Span::styled(e.url.clone(), Style::default().fg(Color::DarkGray)),
        Span::styled(timing, Style::default().fg(Color::DarkGray)),
    ])
}

fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::Green,
        300..=399 => Color::Magenta,
        400..=499 => Color::Yellow,
        500..=599 => Color::Red,
        _ => Color::Gray,
    }
}

fn centered(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}

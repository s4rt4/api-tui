use crate::app::Confirm;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, confirm: Confirm) {
    let (title, prompt) = match confirm {
        Confirm::QuitDirty => (
            " unsaved changes ",
            "You have unsaved changes. Quit anyway?",
        ),
        Confirm::DeleteRequest => (" delete request ", "Delete the selected request?"),
    };

    let area = centered(50, 7, frame.area());

    let keys_line = match confirm {
        Confirm::QuitDirty => Line::from(vec![
            key("y"),
            Span::raw(" discard & quit    "),
            key("w"),
            Span::raw(" save & quit    "),
            key("n"),
            Span::raw(" / "),
            key("Esc"),
            Span::raw(" cancel"),
        ]),
        Confirm::DeleteRequest => Line::from(vec![
            key("y"),
            Span::raw(" yes    "),
            key("n"),
            Span::raw(" / "),
            key("Esc"),
            Span::raw(" cancel"),
        ]),
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::raw(prompt)),
        Line::from(""),
        keys_line,
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn key(label: &str) -> Span<'static> {
    Span::styled(
        label.to_string(),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )
}

fn centered(percent_x: u16, height: u16, r: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(r);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}

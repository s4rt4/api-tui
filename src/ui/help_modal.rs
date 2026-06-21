use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame) {
    let area = centered(60, 70, frame.area());

    let lines = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            key_label("↑/k"),
            Span::raw("    select prev / scroll up"),
        ]),
        Line::from(vec![
            key_label("↓/j"),
            Span::raw("    select next / scroll down"),
        ]),
        Line::from(vec![key_label("Tab"), Span::raw("    cycle panel")]),
        Line::from(vec![key_label("s  "), Span::raw("    send request")]),
        Line::from(vec![key_label("Esc"), Span::raw("    cancel in-flight")]),
        Line::from(vec![
            key_label("h  "),
            Span::raw("    toggle response headers"),
        ]),
        Line::from(vec![
            key_label("e  "),
            Span::raw("    edit field (URL/body/headers/query)"),
        ]),
        Line::from(vec![
            key_label("E  "),
            Span::raw("    edit body in $EDITOR"),
        ]),
        Line::from(vec![key_label("m  "), Span::raw("    cycle method")]),
        Line::from(vec![key_label("a  "), Span::raw("    add request")]),
        Line::from(vec![
            key_label("d  "),
            Span::raw("    delete request (confirm)"),
        ]),
        Line::from(vec![key_label("w  "), Span::raw("    save collection")]),
        Line::from(vec![
            key_label("o  "),
            Span::raw("    export response to file"),
        ]),
        Line::from(vec![
            key_label("y  "),
            Span::raw("    yank response to clipboard"),
        ]),
        Line::from(vec![
            key_label("/  "),
            Span::raw("    search response body"),
        ]),
        Line::from(vec![
            key_label("n/N"),
            Span::raw("    next / prev search match"),
        ]),
        Line::from(vec![
            key_label("H  "),
            Span::raw("    view request history"),
        ]),
        Line::from(vec![key_label("?  "), Span::raw("    toggle this help")]),
        Line::from(vec![
            key_label("q  "),
            Span::raw("    quit (confirm if dirty)"),
        ]),
        Line::from(vec![key_label("^C "), Span::raw("    quit")]),
        Line::from(""),
        Line::from(Span::styled(
            "Headers: \"Key: value\" per line.  Query: \"key=value\" per line.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc, ?, or q to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" help ")
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn key_label(label: &str) -> Span<'static> {
    Span::styled(format!("  {}", label), Style::default().fg(Color::Yellow))
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

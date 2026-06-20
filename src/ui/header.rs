use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let path_str = app
        .collection_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<no file>".into());
    let name = app.collection.name.as_deref().unwrap_or("untitled");
    let dirty = if app.dirty { " ● dirty" } else { "" };

    let line = Line::from(vec![
        Span::styled(
            " ApiTester ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(name.to_string(), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  [{}]  ", path_str),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(format!("env={}", app.env), Style::default().fg(Color::Magenta)),
        Span::styled(dirty.to_string(), Style::default().fg(Color::Yellow)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

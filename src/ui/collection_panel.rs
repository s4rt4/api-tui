use crate::app::{ActivePanel, App};
use crate::ui::style::{method_color, panel_border};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" collections ")
        .border_style(panel_border(app, ActivePanel::CollectionList));

    if app.collection.requests.is_empty() {
        let para = ratatui::widgets::Paragraph::new(Span::styled(
            "  empty — pass a .toml file as arg",
            Style::default().fg(Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .collection
        .requests
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:6}", r.method.to_ascii_uppercase()),
                    Style::default().fg(method_color(&r.method)),
                ),
                Span::raw(" "),
                Span::raw(r.name.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

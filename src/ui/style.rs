use crate::app::{ActivePanel, App};
use ratatui::style::{Color, Style};

pub fn method_color(method: &str) -> Color {
    match method.to_ascii_uppercase().as_str() {
        "GET" => Color::Green,
        "POST" => Color::Yellow,
        "PUT" | "PATCH" => Color::Blue,
        "DELETE" => Color::Red,
        _ => Color::Gray,
    }
}

pub fn panel_border(app: &App, panel: ActivePanel) -> Style {
    if app.active_panel == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

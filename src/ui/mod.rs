pub mod collection_panel;
pub mod confirm_modal;
pub mod header;
pub mod help_modal;
pub mod request_panel;
pub mod response_panel;
pub mod status_bar;
pub mod style;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(3),    // body
        Constraint::Length(1), // status
        Constraint::Length(1), // key hints
    ])
    .split(area);

    header::render(frame, outer[0], app);
    render_body(frame, outer[1], app);
    status_bar::render_status(frame, outer[2], app);
    status_bar::render_keys(frame, outer[3], app);

    if app.help_open {
        help_modal::render(frame);
    }

    if let Some(confirm) = app.confirm {
        confirm_modal::render(frame, confirm);
    }
}

fn render_body(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    collection_panel::render(frame, cols[0], app);

    let right = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    request_panel::render(frame, right[0], app);
    response_panel::render(frame, right[1], app);
}

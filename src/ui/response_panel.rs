use crate::app::{ActivePanel, App};
use crate::http::StatusClass;
use crate::ui::highlight;
use crate::ui::style::panel_border;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" response ")
        .border_style(panel_border(app, ActivePanel::ResponseViewer));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading {
        let para = Paragraph::new(Line::from(Span::styled(
            "  ⏳ Sending... [Esc] to cancel",
            Style::default().fg(Color::Yellow),
        )));
        frame.render_widget(para, inner);
        return;
    }

    let Some(resp) = &app.response else {
        let para = Paragraph::new(Line::from(Span::styled(
            "  No response yet — press [s] to send",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(para, inner);
        return;
    };

    let header_height = if app.show_response_headers {
        ((resp.headers.len() as u16) + 1).min(10)
    } else {
        0
    };

    let mut constraints = vec![Constraint::Length(1)];
    if header_height > 0 {
        constraints.push(Constraint::Length(header_height));
    }
    constraints.push(Constraint::Min(1));

    let chunks = Layout::vertical(constraints).split(inner);

    // Status line
    let status_color = status_class_color(resp.status_class());
    let canonical = canonical_status_text(resp.status);
    let status_line = Line::from(vec![
        Span::styled(
            format!("Status: {} {}", resp.status, canonical),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!("Time: {}ms", resp.elapsed.as_millis()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("   "),
        Span::styled(
            format!("Size: {}", format_bytes(resp.size_bytes())),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("   "),
        Span::styled(
            if app.show_response_headers {
                "[h] hide hdrs"
            } else {
                "[h] show hdrs"
            },
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(status_line), chunks[0]);

    let mut idx = 1;
    if header_height > 0 {
        let header_lines: Vec<Line> = resp
            .headers
            .iter()
            .map(|(k, v)| {
                Line::from(vec![
                    Span::styled(
                        format!("{}: ", k.as_str()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(v.to_str().unwrap_or("?").to_string()),
                ])
            })
            .collect();
        frame.render_widget(
            Paragraph::new(header_lines).wrap(Wrap { trim: false }),
            chunks[idx],
        );
        idx += 1;
    }

    let pretty = resp.pretty_body();
    let mut body_lines: Vec<Line> = if resp.is_json() {
        highlight::highlight_json(&pretty, app.light_theme, app.no_color)
    } else {
        pretty.lines().map(|l| Line::from(l.to_string())).collect()
    };
    highlight_search_matches(&mut body_lines, app);
    let body_para = Paragraph::new(body_lines)
        .scroll((app.response_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(body_para, chunks[idx]);
}

/// Paint a background over body lines that matched the active search; the
/// currently-focused match gets a brighter style than the rest.
fn highlight_search_matches(lines: &mut [Line], app: &App) {
    let Some(search) = &app.search else {
        return;
    };
    let current_line = search.matches.get(search.current).copied();
    for (i, line) in lines.iter_mut().enumerate() {
        if !search.matches.contains(&i) {
            continue;
        }
        let (bg, fg) = if Some(i) == current_line {
            (Color::Yellow, Color::Black)
        } else {
            (Color::DarkGray, Color::White)
        };
        for span in line.spans.iter_mut() {
            span.style = span.style.bg(bg).fg(fg);
        }
    }
}

fn status_class_color(class: StatusClass) -> Color {
    match class {
        StatusClass::Info => Color::Cyan,
        StatusClass::Success => Color::Green,
        StatusClass::Redirect => Color::Magenta,
        StatusClass::ClientError => Color::Yellow,
        StatusClass::ServerError => Color::Red,
        StatusClass::Unknown => Color::Gray,
    }
}

fn canonical_status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

fn format_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    }
}

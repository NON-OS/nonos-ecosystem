mod tabs;
mod overview;
mod network;
mod identities;
mod logs;
mod settings;
mod help;
mod helpers;

pub use tabs::render_tabs;
pub use overview::render_overview;
pub use network::render_network;
pub use identities::render_identities;
pub use logs::render_logs;
pub use settings::render_settings;
pub use help::render_help;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render_ui(f: &mut Frame, app: &mut App) {
    let theme = &app.theme;
    let size = f.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " NONOS Dashboard ",
            Style::default().fg(theme.title).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    f.render_widget(block, size);

    let inner = Rect::new(size.x + 1, size.y + 1, size.width - 2, size.height - 2);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(inner);

    render_tabs(f, app, chunks[0]);

    match app.tab {
        0 => render_overview(f, app, chunks[1]),
        1 => render_network(f, app, chunks[1]),
        2 => render_identities(f, app, chunks[1]),
        3 => render_logs(f, app, chunks[1]),
        4 => render_settings(f, app, chunks[1]),
        _ => {}
    }

    render_status_bar(f, app, chunks[2]);

    if app.show_help {
        render_help(f, app, size);
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let status = ratatui::text::Line::from(vec![
        Span::styled(" [Q]", Style::default().fg(theme.highlight)),
        Span::styled(" Quit ", Style::default().fg(theme.text)),
        Span::styled("[Tab]", Style::default().fg(theme.highlight)),
        Span::styled(" Switch ", Style::default().fg(theme.text)),
        Span::styled("[?]", Style::default().fg(theme.highlight)),
        Span::styled(" Help ", Style::default().fg(theme.text)),
        Span::raw(" | "),
        Span::styled(format!("v{}", VERSION), Style::default().fg(theme.label)),
        Span::raw(" | "),
        Span::styled(
            if data.connected { "CONNECTED" } else { "DISCONNECTED" },
            Style::default().fg(if data.connected { theme.success } else { theme.error }),
        ),
    ]);

    f.render_widget(Paragraph::new(status), area);
}

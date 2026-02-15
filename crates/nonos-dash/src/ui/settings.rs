use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Settings ", Style::default().fg(theme.title)));

    let lines = vec![
        Line::from(Span::styled("Dashboard Settings", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Theme:        ", Style::default().fg(theme.label)),
            Span::styled(&app.theme.name, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  API URL:      ", Style::default().fg(theme.label)),
            Span::styled(&app.api_url, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Refresh:      ", Style::default().fg(theme.label)),
            Span::styled("1s", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Node Configuration", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Data Dir:     ", Style::default().fg(theme.label)),
            Span::styled("~/.nonos", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Config:       ", Style::default().fg(theme.label)),
            Span::styled("~/.nonos/config.toml", Style::default().fg(theme.text)),
        ]),
    ];

    f.render_widget(Paragraph::new(lines).block(block), area);
}

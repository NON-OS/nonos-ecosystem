use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::App;
use super::helpers::centered_rect;

pub fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_area = centered_rect(50, 60, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.highlight))
        .title(Span::styled(" Help ", Style::default().fg(theme.title).add_modifier(Modifier::BOLD)));

    let lines = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1-5      ", Style::default().fg(theme.highlight)),
            Span::styled("Switch to tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Tab      ", Style::default().fg(theme.highlight)),
            Span::styled("Next tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down  ", Style::default().fg(theme.highlight)),
            Span::styled("Scroll", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Actions", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(theme.highlight)),
            Span::styled("Refresh data", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc  ", Style::default().fg(theme.highlight)),
            Span::styled("Quit", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(theme.highlight)),
            Span::styled("Toggle help", Style::default().fg(theme.text)),
        ]),
    ];

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), popup_area);
}

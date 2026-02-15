use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render_identities(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.border));

    let header_text = Line::from(vec![
        Span::styled("ZK Identities: ", Style::default().fg(theme.label)),
        Span::styled(data.identities.len().to_string(), Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::raw("  |  "),
        Span::styled("Press ", Style::default().fg(theme.text)),
        Span::styled("[G]", Style::default().fg(theme.highlight)),
        Span::styled(" to generate new  |  ", Style::default().fg(theme.text)),
        Span::styled("[P]", Style::default().fg(theme.highlight)),
        Span::styled(" to prove", Style::default().fg(theme.text)),
    ]);

    f.render_widget(Paragraph::new(header_text).block(header_block), chunks[0]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Identities ", Style::default().fg(theme.title)));

    if data.identities.is_empty() {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled("No ZK identities found.", Style::default().fg(theme.label))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Generate one with: ", Style::default().fg(theme.text)),
                Span::styled("nonos identity generate", Style::default().fg(theme.highlight)),
            ]),
        ];
        f.render_widget(Paragraph::new(empty_text).block(list_block), chunks[1]);
    } else {
        let items: Vec<ListItem> = data.identities.iter().map(|id| {
            let (icon, style) = if id.registered {
                ("[+]", Style::default().fg(theme.success))
            } else {
                ("[-]", Style::default().fg(theme.label))
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::raw(" "),
                Span::styled(&id.id, Style::default().fg(theme.highlight)),
                Span::raw(" "),
                Span::styled(&id.label, Style::default().fg(theme.text)),
                Span::raw("  "),
                Span::styled(&id.created, Style::default().fg(theme.label)),
            ]))
        }).collect();

        f.render_widget(List::new(items).block(list_block), chunks[1]);
    }
}

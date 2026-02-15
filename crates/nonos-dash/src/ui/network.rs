use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::App;
use crate::globe;

pub fn render_network(f: &mut Frame, app: &mut App, area: Rect) {

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(20), Constraint::Length(8)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[0]);

    globe::render_globe_canvas(f, app, top_chunks[0]);
    render_peer_list(f, app, top_chunks[1]);
    render_globe_stats(f, app, chunks[1]);
}

fn render_peer_list(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(format!(" Peers ({}) ", data.peer_list.len()), Style::default().fg(theme.title)));

    let items: Vec<ListItem> = data.peer_list.iter().take(20).map(|p| {
        let latency_style = if p.latency < 50 {
            Style::default().fg(theme.success)
        } else if p.latency < 200 {
            Style::default().fg(theme.warning)
        } else {
            Style::default().fg(theme.error)
        };

        let location = if p.location.is_empty() {
            "Locating...".to_string()
        } else {
            p.location.clone()
        };

        ListItem::new(Line::from(vec![
            Span::styled(symbols::DOT, Style::default().fg(if p.connected { theme.success } else { theme.error })),
            Span::raw(" "),
            Span::styled(&p.id[..12.min(p.id.len())], Style::default().fg(theme.highlight)),
            Span::raw(" "),
            Span::styled(location, Style::default().fg(theme.label)),
            Span::raw(" "),
            Span::styled(format!("{}ms", p.latency), latency_style),
        ]))
    }).collect();

    f.render_widget(List::new(items).block(block), area);
}

fn render_globe_stats(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Network Statistics ", Style::default().fg(theme.title)));

    let stats_lines = globe::render_globe_stats(app);

    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => {
            f.render_widget(Paragraph::new(stats_lines).block(block), area);
            return;
        }
    };

    let mut lines = stats_lines;
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" ZK Proofs: ", Style::default().fg(theme.label)),
        Span::styled(data.zk_proofs_issued.to_string(), Style::default().fg(theme.highlight)),
        Span::styled("  Cache Hits: ", Style::default().fg(theme.label)),
        Span::styled(data.cache_hits.to_string(), Style::default().fg(theme.highlight)),
        Span::styled("  Tracking Blocked: ", Style::default().fg(theme.label)),
        Span::styled(data.tracking_blocked.to_string(), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
    ]));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

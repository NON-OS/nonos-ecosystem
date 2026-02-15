use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crate::app::App;

pub fn render_logs(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Logs ", Style::default().fg(theme.title)));

    let items: Vec<ListItem> = data.logs.iter().skip(app.log_scroll).map(|log| {
        let level_style = match log.level.as_str() {
            "TRACE" => Style::default().fg(theme.label),
            "DEBUG" => Style::default().fg(theme.text),
            "INFO" => Style::default().fg(theme.success),
            "WARN" => Style::default().fg(theme.warning),
            "ERROR" => Style::default().fg(theme.error),
            _ => Style::default().fg(theme.text),
        };

        ListItem::new(Line::from(vec![
            Span::styled(&log.time, Style::default().fg(theme.label)),
            Span::raw(" "),
            Span::styled(format!("{:5}", log.level), level_style),
            Span::raw(" "),
            Span::styled(&log.target, Style::default().fg(theme.highlight)),
            Span::raw(" "),
            Span::styled(&log.message, Style::default().fg(theme.text)),
        ]))
    }).collect();

    f.render_widget(List::new(items).block(block), area);
}

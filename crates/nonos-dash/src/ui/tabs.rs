use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use crate::app::App;

pub fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let titles = ["Overview", "Network", "Identities", "Logs", "Settings"];

    let tabs: Vec<Line> = titles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.tab as usize {
                Style::default().fg(theme.active_tab).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.inactive_tab)
            };
            Line::from(Span::styled(format!(" {} ", t), style))
        })
        .collect();

    let tabs_widget = Tabs::new(tabs)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(theme.border)))
        .highlight_style(Style::default().fg(theme.active_tab))
        .divider(Span::styled("|", Style::default().fg(theme.border)));

    f.render_widget(tabs_widget, area);
}

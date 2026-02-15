use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
};
use crate::theme::Theme;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn service_line<'a>(name: &'a str, active: bool, theme: &Theme) -> Line<'a> {
    let (icon, style) = if active {
        ("[+]", Style::default().fg(theme.success))
    } else {
        ("[-]", Style::default().fg(theme.error))
    };
    Line::from(vec![
        Span::styled(icon, style),
        Span::raw(" "),
        Span::styled(name, Style::default().fg(theme.text)),
    ])
}

pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, s)
    } else {
        format!("{}m {}s", mins, s)
    }
}

pub fn quality_color(quality: f64, theme: &Theme) -> Color {
    if quality > 0.9 { theme.success }
    else if quality > 0.7 { theme.warning }
    else { theme.error }
}

pub fn tier_color(tier: &str, theme: &Theme) -> Color {
    match tier {
        "Bronze" => Color::Rgb(205, 127, 50),
        "Silver" => Color::Rgb(192, 192, 192),
        "Gold" => Color::Rgb(255, 215, 0),
        "Platinum" => Color::Rgb(229, 228, 226),
        "Diamond" => Color::Rgb(185, 242, 255),
        _ => theme.text,
    }
}

pub fn gauge_color(value: u8, theme: &Theme) -> Color {
    if value < 50 { theme.success }
    else if value < 80 { theme.warning }
    else { theme.error }
}

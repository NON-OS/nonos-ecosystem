use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Wrap},
    Frame,
};
use crate::app::App;
use super::helpers::{format_uptime, quality_color, tier_color, gauge_color, service_line};

pub fn render_overview(f: &mut Frame, app: &mut App, area: Rect) {
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Length(8), Constraint::Min(5)])
        .split(chunks[0]);

    render_status_box(f, app, &data, left_chunks[0]);
    render_sparklines(f, app, &data, left_chunks[1]);
    render_activity(f, app, &data, left_chunks[2]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(5)])
        .split(chunks[1]);

    render_services(f, app, &data, right_chunks[0]);
    render_gauges(f, app, &data, right_chunks[1]);
}

fn render_status_box(f: &mut Frame, app: &App, data: &crate::data::AppData, area: Rect) {
    let theme = &app.theme;

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Node Status ", Style::default().fg(theme.title)));

    let status_text = if data.connected {
        vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.label)),
                Span::styled("ONLINE", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Node ID: ", Style::default().fg(theme.label)),
                Span::styled(&data.node_id[..20.min(data.node_id.len())], Style::default().fg(theme.highlight)),
                Span::styled("...", Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(theme.label)),
                Span::styled(format_uptime(data.uptime_secs), Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Quality: ", Style::default().fg(theme.label)),
                Span::styled(format!("{:.1}%", data.quality_score * 100.0), Style::default().fg(quality_color(data.quality_score, theme))),
            ]),
            Line::from(vec![
                Span::styled("Tier: ", Style::default().fg(theme.label)),
                Span::styled(&data.tier, Style::default().fg(tier_color(&data.tier, theme)).add_modifier(Modifier::BOLD)),
                Span::styled(" | Streak: ", Style::default().fg(theme.label)),
                Span::styled(format!("{} days", data.streak_days), Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Peers: ", Style::default().fg(theme.label)),
                Span::styled(data.peers.to_string(), Style::default().fg(theme.highlight)),
                Span::styled(" | Requests: ", Style::default().fg(theme.label)),
                Span::styled(format!("{}/{}", data.successful_requests, data.total_requests), Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Staked: ", Style::default().fg(theme.label)),
                Span::styled(format!("{:.2} NOX", data.staked_nox), Style::default().fg(theme.highlight)),
            ]),
            Line::from(vec![
                Span::styled("Pending: ", Style::default().fg(theme.label)),
                Span::styled(format!("{:.6} NOX", data.pending_rewards), Style::default().fg(theme.success)),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.label)),
                Span::styled("OFFLINE", Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Start daemon with: ", Style::default().fg(theme.text)),
                Span::styled("nonos run", Style::default().fg(theme.highlight)),
            ]),
        ]
    };

    f.render_widget(Paragraph::new(status_text).block(status_block).wrap(Wrap { trim: true }), area);
}

fn render_sparklines(f: &mut Frame, app: &App, data: &crate::data::AppData, area: Rect) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let req_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Requests/min ", Style::default().fg(theme.title)));

    f.render_widget(
        Sparkline::default().block(req_block).data(&data.request_history).style(Style::default().fg(theme.sparkline)),
        chunks[0],
    );

    let bw_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Bandwidth ", Style::default().fg(theme.title)));

    f.render_widget(
        Sparkline::default().block(bw_block).data(&data.bandwidth_history).style(Style::default().fg(theme.sparkline2)),
        chunks[1],
    );
}

fn render_activity(f: &mut Frame, app: &App, data: &crate::data::AppData, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Recent Activity ", Style::default().fg(theme.title)));

    let items: Vec<ListItem> = data.recent_activity.iter().map(|a| {
        let style = match a.level.as_str() {
            "info" => Style::default().fg(theme.text),
            "warn" => Style::default().fg(theme.warning),
            "error" => Style::default().fg(theme.error),
            _ => Style::default().fg(theme.text),
        };
        ListItem::new(Line::from(vec![
            Span::styled(&a.time, Style::default().fg(theme.label)),
            Span::raw(" "),
            Span::styled(&a.message, style),
        ]))
    }).collect();

    f.render_widget(List::new(items).block(block), area);
}

fn render_services(f: &mut Frame, app: &App, data: &crate::data::AppData, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Services ", Style::default().fg(theme.title)));

    let lines = vec![
        service_line("ZK Identity Engine", data.services.zk_identity, theme),
        service_line("Cache Mixer", data.services.cache_mixer, theme),
        service_line("Tracking Blocker", data.services.tracking_blocker, theme),
        service_line("Stealth Scanner", data.services.stealth_scanner, theme),
        service_line("P2P Network", data.services.p2p_network, theme),
        service_line("Health Beacon", data.services.health_beacon, theme),
        service_line("Quality Oracle", data.services.quality_oracle, theme),
    ];

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_gauges(f: &mut Frame, app: &App, data: &crate::data::AppData, area: Rect) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let cpu = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title(" CPU "))
        .gauge_style(Style::default().fg(gauge_color(data.cpu_usage, theme)))
        .percent(data.cpu_usage as u16)
        .label(format!("{}%", data.cpu_usage));
    f.render_widget(cpu, chunks[0]);

    let mem = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title(" Memory "))
        .gauge_style(Style::default().fg(gauge_color(data.memory_usage, theme)))
        .percent(data.memory_usage as u16)
        .label(format!("{}%", data.memory_usage));
    f.render_widget(mem, chunks[1]);

    let disk = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title(" Disk "))
        .gauge_style(Style::default().fg(gauge_color(data.disk_usage, theme)))
        .percent(data.disk_usage as u16)
        .label(format!("{}%", data.disk_usage));
    f.render_widget(disk, chunks[2]);
}

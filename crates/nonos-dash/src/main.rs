//! NONOS Dashboard 
//! Usage: nonos-dash [OPTIONS]
//!
//! Options:
//!   --theme <THEME>  Dashboard theme (matrix, dark, light) [default: matrix]
//!   --api-url <URL>  API endpoint [default: http://127.0.0.1:8420]


use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tokio::time::interval;

mod app;
mod globe;
mod theme;

use app::App;
use theme::Theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "nonos-dash")]
#[command(about = "NONOS Dashboard - Nyx-style TUI for network monitoring")]
#[command(version = VERSION)]
struct Cli {
    /// Dashboard theme (matrix, dark, light)
    #[arg(long, default_value = "matrix")]
    theme: String,

    /// API endpoint for the running daemon
    #[arg(long, default_value = "http://127.0.0.1:8420")]
    api_url: String,

    /// Disable mouse capture
    #[arg(long)]
    no_mouse: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if !cli.no_mouse {
        execute!(stdout, EnableMouseCapture)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let theme = Theme::from_name(&cli.theme);
    let mut app = App::new(cli.api_url, theme);

    // Run the app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    if !cli.no_mouse {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    // Spawn the data refresh task - fetches from multiple endpoints
    let api_url = app.api_url.clone();
    let data = app.data.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        let client = reqwest::Client::new();

        loop {
            interval.tick().await;

            // Fetch from all endpoints in parallel
            let (status_res, peers_res, privacy_res) = tokio::join!(
                fetch_endpoint(&client, &api_url, "/api/status"),
                fetch_endpoint(&client, &api_url, "/api/peers"),
                fetch_endpoint(&client, &api_url, "/api/privacy/stats"),
            );

            // Get system resources in a blocking task to avoid runtime issues
            let resources = tokio::task::spawn_blocking(|| {
                use sysinfo::{System, CpuRefreshKind, MemoryRefreshKind, Disks};

                let mut sys = System::new();
                sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
                sys.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());

                let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                    / sys.cpus().len().max(1) as f32;
                let cpu = cpu_usage.min(100.0) as u8;

                let total_mem = sys.total_memory();
                let used_mem = sys.used_memory();
                let memory = if total_mem > 0 {
                    ((used_mem as f64 / total_mem as f64) * 100.0) as u8
                } else {
                    0
                };

                let mut disk = 0u8;
                let disks = Disks::new_with_refreshed_list();
                for d in disks.list() {
                    let total = d.total_space();
                    let available = d.available_space();
                    if total > 0 {
                        let used = total - available;
                        disk = ((used as f64 / total as f64) * 100.0) as u8;
                        break;
                    }
                }

                (cpu, memory, disk)
            }).await.unwrap_or((0, 0, 0));

            let mut data = data.write().await;

            // Update status (main metrics)
            if let Ok(stats) = status_res {
                data.update(stats);
            } else {
                data.connected = false;
            }

            // Update peers with real peer list
            if let Ok(peers) = peers_res {
                if let Ok(peers_resp) = serde_json::from_value::<app::PeersApiResponse>(peers) {
                    data.update_peers(peers_resp);
                }
            }

            // Update privacy stats
            if let Ok(privacy) = privacy_res {
                if let Ok(privacy_resp) = serde_json::from_value::<app::PrivacyStatsApiResponse>(privacy) {
                    data.update_privacy_stats(privacy_resp);
                }
            }

            // Update system resource usage
            data.update_resources(resources.0, resources.1, resources.2);
        }
    });

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('1') => app.tab = 0,
                        KeyCode::Char('2') => app.tab = 1,
                        KeyCode::Char('3') => app.tab = 2,
                        KeyCode::Char('4') => app.tab = 3,
                        KeyCode::Char('5') => app.tab = 4,
                        KeyCode::Tab => app.tab = (app.tab + 1) % 5,
                        KeyCode::BackTab => app.tab = (app.tab + 4) % 5,
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Char('?') => app.show_help = !app.show_help,
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let theme = &app.theme;
    let size = f.area();

    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " NONOS Dashboard ",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    f.render_widget(block, size);

    // Main layout
    let inner = Rect::new(size.x + 1, size.y + 1, size.width - 2, size.height - 2);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(inner);

    // Render tabs
    render_tabs(f, app, chunks[0]);

    // Render content based on selected tab
    match app.tab {
        0 => render_overview(f, app, chunks[1]),
        1 => render_network(f, app, chunks[1]),
        2 => render_identities(f, app, chunks[1]),
        3 => render_logs(f, app, chunks[1]),
        4 => render_settings(f, app, chunks[1]),
        _ => {}
    }

    // Render status bar
    render_status_bar(f, app, chunks[2]);

    // Render help overlay if enabled
    if app.show_help {
        render_help(f, app, size);
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let titles = vec!["Overview", "Network", "Identities", "Logs", "Settings"];
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

    let tabs = Tabs::new(tabs)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(theme.border)))
        .highlight_style(Style::default().fg(theme.active_tab))
        .divider(Span::styled("|", Style::default().fg(theme.border)));

    f.render_widget(tabs, area);
}

fn render_overview(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return, // Skip render if lock unavailable
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left side: Stats and graphs
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Node status (expanded)
            Constraint::Length(8),  // Sparklines
            Constraint::Min(5),     // Activity log
        ])
        .split(chunks[0]);

    // Node status box
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
            Line::from(vec![
                Span::styled("", Style::default()),
            ]),
            Line::from(vec![
                Span::styled("Start daemon with: ", Style::default().fg(theme.text)),
                Span::styled("nonos run", Style::default().fg(theme.highlight)),
            ]),
        ]
    };

    let status_para = Paragraph::new(status_text)
        .block(status_block)
        .wrap(Wrap { trim: true });
    f.render_widget(status_para, left_chunks[0]);

    // Sparklines
    let sparkline_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left_chunks[1]);

    // Requests sparkline
    let requests_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Requests/min ", Style::default().fg(theme.title)));
    let requests_sparkline = Sparkline::default()
        .block(requests_block)
        .data(&data.request_history)
        .style(Style::default().fg(theme.sparkline));
    f.render_widget(requests_sparkline, sparkline_chunks[0]);

    // Bandwidth sparkline
    let bandwidth_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Bandwidth ", Style::default().fg(theme.title)));
    let bandwidth_sparkline = Sparkline::default()
        .block(bandwidth_block)
        .data(&data.bandwidth_history)
        .style(Style::default().fg(theme.sparkline2));
    f.render_widget(bandwidth_sparkline, sparkline_chunks[1]);

    // Activity log
    let activity_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Recent Activity ", Style::default().fg(theme.title)));
    let activity_items: Vec<ListItem> = data.recent_activity
        .iter()
        .map(|a| {
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
        })
        .collect();
    let activity_list = List::new(activity_items).block(activity_block);
    f.render_widget(activity_list, left_chunks[2]);

    // Right side: Services and Gauges
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Services
            Constraint::Min(5),     // Gauges
        ])
        .split(chunks[1]);

    // Services status
    let services_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Services ", Style::default().fg(theme.title)));
    let services_text = vec![
        service_line("ZK Identity Engine", data.services.zk_identity, theme),
        service_line("Cache Mixer", data.services.cache_mixer, theme),
        service_line("Tracking Blocker", data.services.tracking_blocker, theme),
        service_line("Stealth Scanner", data.services.stealth_scanner, theme),
        service_line("P2P Network", data.services.p2p_network, theme),
        service_line("Health Beacon", data.services.health_beacon, theme),
        service_line("Quality Oracle", data.services.quality_oracle, theme),
    ];
    let services_para = Paragraph::new(services_text).block(services_block);
    f.render_widget(services_para, right_chunks[0]);

    // Gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(right_chunks[1]);

    // CPU gauge
    let cpu_gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(" CPU ", Style::default().fg(theme.title))))
        .gauge_style(Style::default().fg(gauge_color(data.cpu_usage, theme)))
        .percent(data.cpu_usage as u16)
        .label(format!("{}%", data.cpu_usage));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    // Memory gauge
    let mem_gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(" Memory ", Style::default().fg(theme.title))))
        .gauge_style(Style::default().fg(gauge_color(data.memory_usage, theme)))
        .percent(data.memory_usage as u16)
        .label(format!("{}%", data.memory_usage));
    f.render_widget(mem_gauge, gauge_chunks[1]);

    // Disk gauge
    let disk_gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(" Disk ", Style::default().fg(theme.title))))
        .gauge_style(Style::default().fg(gauge_color(data.disk_usage, theme)))
        .percent(data.disk_usage as u16)
        .label(format!("{}%", data.disk_usage));
    f.render_widget(disk_gauge, gauge_chunks[2]);
}

fn render_network(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: 3D Globe visualization with proper Canvas rendering
    globe::render_globe_canvas(f, app, chunks[0]);

    // Right: Peer list
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };
    let peers_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(format!(" Peers ({}) ", data.peer_list.len()), Style::default().fg(theme.title)));

    let peer_items: Vec<ListItem> = data.peer_list
        .iter()
        .take(20)
        .map(|p| {
            let latency_style = if p.latency < 50 {
                Style::default().fg(theme.success)
            } else if p.latency < 200 {
                Style::default().fg(theme.warning)
            } else {
                Style::default().fg(theme.error)
            };
            ListItem::new(Line::from(vec![
                Span::styled(symbols::DOT, Style::default().fg(if p.connected { theme.success } else { theme.error })),
                Span::raw(" "),
                Span::styled(&p.id[..12.min(p.id.len())], Style::default().fg(theme.highlight)),
                Span::raw(" "),
                Span::styled(&p.location, Style::default().fg(theme.label)),
                Span::raw(" "),
                Span::styled(format!("{}ms", p.latency), latency_style),
            ]))
        })
        .collect();

    let peers_list = List::new(peer_items).block(peers_block);
    f.render_widget(peers_list, chunks[1]);
}

fn render_identities(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // List
        ])
        .split(area);

    // Header
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
    let header = Paragraph::new(header_text).block(header_block);
    f.render_widget(header, chunks[0]);

    // Identity list
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
        let empty_para = Paragraph::new(empty_text).block(list_block);
        f.render_widget(empty_para, chunks[1]);
    } else {
        let id_items: Vec<ListItem> = data.identities
            .iter()
            .map(|id| {
                let registered_icon = if id.registered { "[+]" } else { "[-]" };
                let registered_style = if id.registered {
                    Style::default().fg(theme.success)
                } else {
                    Style::default().fg(theme.label)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(registered_icon, registered_style),
                    Span::raw(" "),
                    Span::styled(&id.id, Style::default().fg(theme.highlight)),
                    Span::raw(" "),
                    Span::styled(&id.label, Style::default().fg(theme.text)),
                    Span::raw("  "),
                    Span::styled(&id.created, Style::default().fg(theme.label)),
                ]))
            })
            .collect();

        let id_list = List::new(id_items).block(list_block);
        f.render_widget(id_list, chunks[1]);
    }
}

fn render_logs(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Logs ", Style::default().fg(theme.title)));

    let log_items: Vec<ListItem> = data.logs
        .iter()
        .skip(app.log_scroll)
        .map(|log| {
            let level_style = match log.level.as_str() {
                "TRACE" => Style::default().fg(theme.label),
                "DEBUG" => Style::default().fg(theme.text),
                "INFO" => Style::default().fg(theme.success),
                "WARN" => Style::default().fg(theme.warning),
                "ERROR" => Style::default().fg(theme.error),
                _ => Style::default().fg(theme.text),
            };
            let level_text = format!("{:5}", log.level);
            ListItem::new(Line::from(vec![
                Span::styled(&log.time, Style::default().fg(theme.label)),
                Span::raw(" "),
                Span::styled(level_text, level_style),
                Span::raw(" "),
                Span::styled(&log.target, Style::default().fg(theme.highlight)),
                Span::raw(" "),
                Span::styled(&log.message, Style::default().fg(theme.text)),
            ]))
        })
        .collect();

    let log_list = List::new(log_items).block(block);
    f.render_widget(log_list, area);
}

fn render_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Settings ", Style::default().fg(theme.title)));

    let settings_text = vec![
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

    let settings_para = Paragraph::new(settings_text).block(block);
    f.render_widget(settings_para, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let data = match app.data.try_read() {
        Ok(d) => d,
        Err(_) => return,
    };

    let status = Line::from(vec![
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

    let status_bar = Paragraph::new(status);
    f.render_widget(status_bar, area);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let popup_area = centered_rect(50, 60, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.highlight))
        .title(Span::styled(" Help ", Style::default().fg(theme.title).add_modifier(Modifier::BOLD)));

    let help_text = vec![
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

    let help_para = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });

    // Clear the area first
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(help_para, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn service_line<'a>(name: &'a str, active: bool, theme: &Theme) -> Line<'a> {
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

fn format_uptime(secs: u64) -> String {
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

fn quality_color(quality: f64, theme: &Theme) -> Color {
    if quality > 0.9 { theme.success }
    else if quality > 0.7 { theme.warning }
    else { theme.error }
}

fn tier_color(tier: &str, theme: &Theme) -> Color {
    match tier {
        "Bronze" => Color::Rgb(205, 127, 50),
        "Silver" => Color::Rgb(192, 192, 192),
        "Gold" => Color::Rgb(255, 215, 0),
        "Platinum" => Color::Rgb(229, 228, 226),
        "Diamond" => Color::Rgb(185, 242, 255),
        _ => theme.text,
    }
}

fn gauge_color(value: u8, theme: &Theme) -> Color {
    if value < 50 { theme.success }
    else if value < 80 { theme.warning }
    else { theme.error }
}

/// Fetch from a specific API endpoint
async fn fetch_endpoint(client: &reqwest::Client, api_url: &str, endpoint: &str) -> Result<serde_json::Value> {
    let resp = client
        .get(format!("{}{}", api_url, endpoint))
        .timeout(Duration::from_secs(2))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}


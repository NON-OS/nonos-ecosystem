// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use ratatui::style::{Color, Style};
use ratatui::widgets::canvas::{Canvas, Circle, Points};
use ratatui::widgets::Block;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::f64::consts::PI;
use crate::app::App;

const EARTH_RADIUS: f64 = 1.0;
const GLOBE_SCALE: f64 = 40.0;

#[derive(Clone, Debug)]
pub struct GeoNode {
    pub lat: f64,
    pub lon: f64,
    pub is_bootstrap: bool,
}

pub struct Globe {
    pub rotation_y: f64,
    pub rotation_x: f64,
    pub nodes: Vec<GeoNode>,
}

impl Default for Globe {
    fn default() -> Self {
        Self {
            rotation_y: 0.0,
            rotation_x: 0.2,
            nodes: Vec::new(),
        }
    }
}

impl Globe {
    pub fn rotate(&mut self, delta: f64) {
        self.rotation_y = (self.rotation_y + delta) % (2.0 * PI);
    }
}

fn geo_to_3d(lat: f64, lon: f64, rotation_y: f64, rotation_x: f64) -> (f64, f64, f64) {
    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians() + rotation_y;

    let x = EARTH_RADIUS * lat_rad.cos() * lon_rad.cos();
    let y = EARTH_RADIUS * lat_rad.cos() * lon_rad.sin();
    let z = EARTH_RADIUS * lat_rad.sin();

    let cos_rx = rotation_x.cos();
    let sin_rx = rotation_x.sin();
    let y_rot = y * cos_rx - z * sin_rx;
    let z_rot = y * sin_rx + z * cos_rx;

    (x, y_rot, z_rot)
}

fn project_3d_to_2d(x: f64, y: f64, z: f64) -> Option<(f64, f64)> {
    if y < -0.1 {
        return None;
    }
    let scale = GLOBE_SCALE / (2.0 + y);
    Some((x * scale, z * scale))
}

fn generate_sphere_points(rotation_y: f64, rotation_x: f64) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    for lat in (-80..=80).step_by(10) {
        for lon in (0..360).step_by(5) {
            let (x, y, z) = geo_to_3d(lat as f64, lon as f64, rotation_y, rotation_x);
            if let Some((px, py)) = project_3d_to_2d(x, y, z) {
                points.push((px, py));
            }
        }
    }

    for lon in (0..360).step_by(30) {
        for lat in (-80..=80).step_by(5) {
            let (x, y, z) = geo_to_3d(lat as f64, lon as f64, rotation_y, rotation_x);
            if let Some((px, py)) = project_3d_to_2d(x, y, z) {
                points.push((px, py));
            }
        }
    }

    points
}

fn generate_continent_outline(rotation_y: f64, rotation_x: f64) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    // Simplified continent coordinates
    let continents: &[(f64, f64)] = &[
        // Europe
        (52.0, 5.0), (48.0, 2.0), (43.0, -9.0), (36.0, -6.0), (37.0, -25.0),
        (35.0, 25.0), (32.0, 35.0), (42.0, 45.0), (55.0, 38.0), (60.0, 30.0),
        (70.0, 25.0), (65.0, -20.0), (58.0, 10.0), (52.0, 5.0),
        // Africa
        (35.0, -5.0), (32.0, 32.0), (5.0, 43.0), (-5.0, 39.0), (-15.0, 40.0),
        (-25.0, 47.0), (-34.0, 25.0), (-35.0, 18.0), (-29.0, 16.0),
        (-22.0, 14.0), (-12.0, 14.0), (5.0, -10.0), (15.0, -17.0),
        (25.0, -15.0), (35.0, -5.0),
        // Asia
        (55.0, 60.0), (45.0, 85.0), (35.0, 105.0), (22.0, 120.0),
        (5.0, 105.0), (-8.0, 110.0), (-20.0, 118.0), (-32.0, 115.0),
        (-38.0, 145.0), (-45.0, 170.0), (-35.0, 174.0), (-47.0, 167.0),
        // North America
        (50.0, -130.0), (35.0, -120.0), (25.0, -110.0), (20.0, -105.0),
        (15.0, -90.0), (10.0, -84.0), (8.0, -80.0),
        // South America
        (10.0, -75.0), (5.0, -77.0), (-5.0, -80.0), (-18.0, -70.0),
        (-35.0, -72.0), (-55.0, -70.0), (-55.0, -65.0), (-45.0, -65.0),
        (-35.0, -57.0), (-23.0, -43.0), (-5.0, -35.0), (5.0, -50.0),
        (10.0, -62.0), (10.0, -75.0),
        // Canada
        (45.0, -65.0), (48.0, -55.0), (60.0, -65.0), (70.0, -70.0),
        (75.0, -95.0), (70.0, -130.0), (60.0, -140.0), (55.0, -130.0),
        (50.0, -130.0),
        // Australia
        (-25.0, 115.0), (-20.0, 150.0), (-35.0, 150.0), (-38.0, 145.0),
        (-35.0, 137.0), (-32.0, 115.0), (-25.0, 115.0),
    ];

    for &(lat, lon) in continents {
        let (x, y, z) = geo_to_3d(lat, lon, rotation_y, rotation_x);
        if let Some((px, py)) = project_3d_to_2d(x, y, z) {
            points.push((px, py));
        }
    }

    points
}

pub fn render_globe_canvas(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let mut data = match app.data.try_write() {
        Ok(d) => d,
        Err(_) => return,
    };

    data.globe.rotate(0.01);
    let globe = &data.globe;

    let sphere_points = generate_sphere_points(globe.rotation_y, globe.rotation_x);
    let continent_points = generate_continent_outline(globe.rotation_y, globe.rotation_x);

    let mut node_points: Vec<(f64, f64, Color, bool)> = Vec::new();

    for node in &globe.nodes {
        let (x, y, z) = geo_to_3d(node.lat, node.lon, globe.rotation_y, globe.rotation_x);
        if let Some((px, py)) = project_3d_to_2d(x, y, z) {
            let color = if node.is_bootstrap {
                theme.success
            } else {
                theme.highlight
            };
            node_points.push((px, py, color, node.is_bootstrap));
        }
    }

    let canvas = Canvas::default()
        .block(Block::bordered().title(" Network Globe ").border_style(Style::default().fg(theme.border)))
        .x_bounds([-50.0, 50.0])
        .y_bounds([-30.0, 30.0])
        .paint(move |ctx| {
            ctx.draw(&Points {
                coords: &sphere_points,
                color: Color::Rgb(30, 30, 40),
            });

            ctx.draw(&Points {
                coords: &continent_points,
                color: Color::Rgb(50, 80, 50),
            });

            for &(x, y, color, is_bootstrap) in &node_points {
                if is_bootstrap {
                    ctx.draw(&Circle {
                        x,
                        y,
                        radius: 1.5,
                        color,
                    });
                } else {
                    ctx.draw(&Points {
                        coords: &[(x, y)],
                        color,
                    });
                }
            }
        });

    f.render_widget(canvas, area);
}

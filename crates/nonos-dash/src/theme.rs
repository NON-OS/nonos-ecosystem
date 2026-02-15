//! Theme configuration for the NONOS Dashboard

use ratatui::style::Color;

/// Dashboard theme configuration
#[derive(Clone)]
pub struct Theme {
    /// Theme name
    pub name: String,

    // General colors
    pub border: Color,
    pub title: Color,
    pub text: Color,
    pub label: Color,
    pub highlight: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,

    // Tab colors
    pub active_tab: Color,
    pub inactive_tab: Color,

    // Chart colors
    pub sparkline: Color,
    pub sparkline2: Color,

    /// Background color (used when terminal supports it)
    pub _background: Color,
}

impl Theme {
    /// Create a theme from a name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "dark" => Self::dark(),
            "light" => Self::light(),
            "matrix" | _ => Self::matrix(),
        }
    }

    /// Matrix theme (default) - green on black, hacker aesthetic
    pub fn matrix() -> Self {
        Self {
            name: "matrix".to_string(),
            border: Color::Rgb(0, 100, 0),        // Dark green
            title: Color::Rgb(0, 255, 0),         // Bright green
            text: Color::Rgb(0, 200, 0),          // Medium green
            label: Color::Rgb(0, 128, 0),         // Darker green
            highlight: Color::Rgb(0, 255, 127),   // Spring green
            success: Color::Rgb(0, 255, 0),       // Bright green
            warning: Color::Rgb(255, 255, 0),     // Yellow
            error: Color::Rgb(255, 0, 0),         // Red
            active_tab: Color::Rgb(0, 255, 0),    // Bright green
            inactive_tab: Color::Rgb(0, 100, 0),  // Dark green
            sparkline: Color::Rgb(0, 255, 127),   // Spring green
            sparkline2: Color::Rgb(0, 200, 200),  // Cyan-ish
            _background: Color::Black,
        }
    }

    /// Dark theme - blue/cyan accents
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            border: Color::Rgb(70, 70, 80),
            title: Color::Rgb(100, 200, 255),     // Light blue
            text: Color::Rgb(200, 200, 200),      // Light gray
            label: Color::Rgb(128, 128, 140),     // Gray
            highlight: Color::Rgb(0, 191, 255),   // Deep sky blue
            success: Color::Rgb(50, 205, 50),     // Lime green
            warning: Color::Rgb(255, 165, 0),     // Orange
            error: Color::Rgb(255, 69, 0),        // Red-orange
            active_tab: Color::Rgb(0, 191, 255),  // Deep sky blue
            inactive_tab: Color::Rgb(100, 100, 110),
            sparkline: Color::Rgb(0, 191, 255),
            sparkline2: Color::Rgb(138, 43, 226), // Blue-violet
            _background: Color::Rgb(20, 20, 25),
        }
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            border: Color::Rgb(180, 180, 190),
            title: Color::Rgb(30, 30, 40),
            text: Color::Rgb(50, 50, 60),
            label: Color::Rgb(100, 100, 110),
            highlight: Color::Rgb(0, 120, 215),   // Windows blue
            success: Color::Rgb(0, 128, 0),       // Green
            warning: Color::Rgb(200, 130, 0),     // Dark orange
            error: Color::Rgb(200, 0, 0),         // Dark red
            active_tab: Color::Rgb(0, 120, 215),
            inactive_tab: Color::Rgb(140, 140, 150),
            sparkline: Color::Rgb(0, 120, 215),
            sparkline2: Color::Rgb(128, 0, 128),  // Purple
            _background: Color::Rgb(250, 250, 250),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::matrix()
    }
}

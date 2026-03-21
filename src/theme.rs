use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,       // titles, labels, section headers
    pub active: Color,       // shortcuts, active field, highlighted items
    pub success: Color,      // passwords visible, confirmations, checkboxes
    pub error: Color,        // errors, delete warnings
    pub muted: Color,        // hints, secondary text, dates
    pub tag: Color,          // tags
    pub folder: Color,       // folder icons and names
    pub highlight_bg: Color, // list selection background
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            accent: Color::Cyan,
            active: Color::Yellow,
            success: Color::Green,
            error: Color::Red,
            muted: Color::DarkGray,
            tag: Color::Magenta,
            folder: Color::Yellow,
            highlight_bg: Color::DarkGray,
        }
    }

    pub fn light() -> Self {
        Self {
            accent: Color::Blue,
            active: Color::Red,
            success: Color::Green,
            error: Color::LightRed,
            muted: Color::Gray,
            tag: Color::Magenta,
            folder: Color::Blue,
            highlight_bg: Color::Indexed(254), // light gray
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }
}

pub const THEME_OPTIONS: &[&str] = &["dark", "light"];

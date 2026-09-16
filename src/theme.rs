use ratatui::style::Color;

fn parse_hex(hex: &str) -> Color {
    let h = hex.trim().trim_start_matches('#');
    let s: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.to_string()
    };
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::Reset
}

#[derive(Clone, Copy)]
pub struct Theme {
    /// Box borders
    pub border:       Color,
    /// Block/widget titles
    pub title:        Color,
    /// Directory entries
    pub dir:          Color,
    /// Normal body text
    pub text:         Color,
    /// De-emphasised / comment text
    pub dim:          Color,
    /// Selected-row background
    pub highlight_bg: Color,
    /// Selected-row foreground
    pub highlight_fg: Color,
    /// "Yes" / confirm / good state
    pub yes:          Color,
    /// "No"  / danger / bad state
    pub no:           Color,
    /// Keyboard key labels in the controls bar
    pub selector:     Color,
    /// Size bar fill
    pub bar:          Color,
    /// File / directory sizes
    pub size:         Color,
    /// Scanning spinner / transient status
    pub scanning:     Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Catppuccin Mocha — matches tuipaly defaults exactly
        Self {
            border:       parse_hex("#cba6f7"), // mauve
            title:        parse_hex("#f5c2e7"), // pink
            dir:          parse_hex("#89b4fa"), // blue
            text:         parse_hex("#cdd6f4"), // lavender
            dim:          parse_hex("#6c7086"), // overlay0
            highlight_bg: parse_hex("#313244"), // surface0
            highlight_fg: parse_hex("#cba6f7"), // mauve
            yes:          parse_hex("#a6e3a1"), // green
            no:           parse_hex("#f38ba8"), // red
            selector:     parse_hex("#f9e2af"), // yellow
            bar:          parse_hex("#a6e3a1"), // green
            size:         parse_hex("#fab387"), // peach
            scanning:     parse_hex("#f9e2af"), // yellow
        }
    }
}

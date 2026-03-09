//! Color theme definitions for the terminal UI.

use ratatui::style::Color;

/// A color theme for the SmartLog UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Human-readable theme name.
    pub name: &'static str,
    /// Color for ERROR level logs.
    pub error: Color,
    /// Color for WARN level logs.
    pub warn: Color,
    /// Color for INFO level logs.
    pub info: Color,
    /// Color for DEBUG level logs.
    pub debug: Color,
    /// Color for UNKNOWN level logs.
    pub unknown: Color,
    /// Foreground color for search highlights.
    pub highlight_fg: Color,
    /// Background color for search highlights.
    pub highlight_bg: Color,
    /// Color for the input bar when in editing mode.
    pub input_active: Color,
    /// Color for source labels in multi-file mode.
    pub source_color: Color,
    /// Color for relative timestamp prefixes.
    pub timestamp_color: Color,
}

impl Theme {
    /// Dark theme (default) — designed for dark terminal backgrounds.
    pub const DARK: Theme = Theme {
        name: "dark",
        error: Color::Red,
        warn: Color::Yellow,
        info: Color::Green,
        debug: Color::Blue,
        unknown: Color::White,
        highlight_fg: Color::Black,
        highlight_bg: Color::Cyan,
        input_active: Color::Yellow,
        source_color: Color::Magenta,
        timestamp_color: Color::DarkGray,
    };

    /// Light theme — designed for light terminal backgrounds.
    pub const LIGHT: Theme = Theme {
        name: "light",
        error: Color::Red,
        warn: Color::Yellow,
        info: Color::Green,
        debug: Color::Blue,
        unknown: Color::DarkGray,
        highlight_fg: Color::White,
        highlight_bg: Color::DarkGray,
        input_active: Color::Yellow,
        source_color: Color::Magenta,
        timestamp_color: Color::Gray,
    };

    /// Solarized theme — based on the Solarized color palette.
    pub const SOLARIZED: Theme = Theme {
        name: "solarized",
        error: Color::Red,
        warn: Color::Yellow,
        info: Color::Cyan,
        debug: Color::Blue,
        unknown: Color::White,
        highlight_fg: Color::White,
        highlight_bg: Color::Magenta,
        input_active: Color::Cyan,
        source_color: Color::Green,
        timestamp_color: Color::DarkGray,
    };

    /// Dracula theme — based on the Dracula color palette.
    pub const DRACULA: Theme = Theme {
        name: "dracula",
        error: Color::LightRed,
        warn: Color::LightYellow,
        info: Color::LightGreen,
        debug: Color::LightCyan,
        unknown: Color::White,
        highlight_fg: Color::Black,
        highlight_bg: Color::LightMagenta,
        input_active: Color::LightMagenta,
        source_color: Color::LightCyan,
        timestamp_color: Color::DarkGray,
    };

    /// Returns all available theme presets.
    pub fn all() -> &'static [Theme] {
        static PRESETS: [Theme; 4] = [Theme::DARK, Theme::LIGHT, Theme::SOLARIZED, Theme::DRACULA];
        &PRESETS
    }

    /// Returns the theme matching the given name (case-insensitive), defaulting to DARK.
    pub fn by_name(name: &str) -> Theme {
        Self::all()
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
            .copied()
            .unwrap_or(Self::DARK)
    }

    /// Returns the next theme in the preset cycle.
    pub fn next(self) -> Theme {
        let presets = Self::all();
        let idx = presets
            .iter()
            .position(|t| t.name == self.name)
            .unwrap_or(0);
        presets[(idx + 1) % presets.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_returns_four_presets() {
        assert_eq!(Theme::all().len(), 4);
    }

    #[test]
    fn test_by_name_case_insensitive() {
        assert_eq!(Theme::by_name("DARK"), Theme::DARK);
        assert_eq!(Theme::by_name("Dark"), Theme::DARK);
        assert_eq!(Theme::by_name("solarized"), Theme::SOLARIZED);
    }

    #[test]
    fn test_by_name_unknown_defaults_to_dark() {
        assert_eq!(Theme::by_name("nonexistent"), Theme::DARK);
    }

    #[test]
    fn test_next_cycles_through_all() {
        let mut theme = Theme::DARK;
        theme = theme.next();
        assert_eq!(theme, Theme::LIGHT);
        theme = theme.next();
        assert_eq!(theme, Theme::SOLARIZED);
        theme = theme.next();
        assert_eq!(theme, Theme::DRACULA);
        theme = theme.next();
        assert_eq!(theme, Theme::DARK);
    }

    #[test]
    fn test_dark_theme_colors() {
        assert_eq!(Theme::DARK.error, Color::Red);
        assert_eq!(Theme::DARK.info, Color::Green);
        assert_eq!(Theme::DARK.highlight_bg, Color::Cyan);
    }

    #[test]
    fn test_theme_presets_have_unique_names() {
        let names: Vec<_> = Theme::all().iter().map(|t| t.name).collect();
        for (i, name) in names.iter().enumerate() {
            assert!(!names[i + 1..].contains(name), "Duplicate name: {name}");
        }
    }
}

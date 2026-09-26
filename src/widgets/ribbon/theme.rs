//! Ribbon-specific themes matching the 10 SARibbon built-in styles.
//!
//! Each theme defines the ribbon's visual identity — tab bar background, group
//! background, accent colors, borders — while the rest of the application uses
//! the global [`Theme`](crate::Theme). This separation lets the ribbon adopt a
//! different visual language (Office 2013 flat, Office 2021 rounded, Windows 7
//! Aero, etc.) without affecting the rest of the UI.
//!
//! The 10 themes, matching SARibbon's built-in set:
//!
//! - **Office 2013**: pure white, flat design, light gray groups
//! - **Office 2016 Blue/Green/Dark**: white or dark background with colored title bar
//! - **Office 2021 Blue/Green/Dark**: refined 2016 with adjusted accents
//! - **Windows 7**: Aero glass effect with blue/silver tones
//! - **Dark / Dark2**: dark variants with different accent hues

use iced::Color;

/// The 10 built-in ribbon themes, matching SARibbon's set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonTheme {
    /// Office 2013: pure white, flat, light gray groups, blue accent.
    #[default]
    Office2013,
    /// Office 2016 Blue: white background, blue title bar accent.
    Office2016Blue,
    /// Office 2016 Green: white background, green title bar accent.
    Office2016Green,
    /// Office 2016 Dark: dark background, blue accent.
    Office2016Dark,
    /// Office 2021 Blue: refined 2016, white background, blue accent.
    Office2021Blue,
    /// Office 2021 Green: refined 2016, white background, green accent.
    Office2021Green,
    /// Office 2021 Dark: refined 2016, dark background, blue accent.
    Office2021Dark,
    /// Windows 7: Aero glass, blue/silver tones.
    Windows7,
    /// Dark: dark background, blue accent.
    Dark,
    /// Dark2: dark background, teal/cyan accent.
    Dark2,
}

impl RibbonTheme {
    /// Every theme, in the order a selector would list them.
    pub const ALL: &'static [RibbonTheme] = &[
        Self::Office2013,
        Self::Office2016Blue,
        Self::Office2016Green,
        Self::Office2016Dark,
        Self::Office2021Blue,
        Self::Office2021Green,
        Self::Office2021Dark,
        Self::Windows7,
        Self::Dark,
        Self::Dark2,
    ];

    /// The label for a theme, for a settings control.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Office2013 => "Office 2013",
            Self::Office2016Blue => "Office 2016 Blue",
            Self::Office2016Green => "Office 2016 Green",
            Self::Office2016Dark => "Office 2016 Dark",
            Self::Office2021Blue => "Office 2021 Blue",
            Self::Office2021Green => "Office 2021 Green",
            Self::Office2021Dark => "Office 2021 Dark",
            Self::Windows7 => "Windows 7",
            Self::Dark => "Dark",
            Self::Dark2 => "Dark 2",
        }
    }

    /// The accent color for this theme — the primary identity color.
    #[must_use]
    pub fn accent(self) -> Color {
        match self {
            Self::Office2013 => rgb8(0x2b, 0x57, 0x9a),
            Self::Office2016Blue => rgb8(0x2b, 0x57, 0x9a),
            Self::Office2016Green => rgb8(0x10, 0x7c, 0x41),
            Self::Office2016Dark => rgb8(0x2b, 0x57, 0x9a),
            Self::Office2021Blue => rgb8(0x0f, 0x6c, 0xbf),
            Self::Office2021Green => rgb8(0x10, 0x7c, 0x41),
            Self::Office2021Dark => rgb8(0x0f, 0x6c, 0xbf),
            Self::Windows7 => rgb8(0x3b, 0x8b, 0xc2),
            Self::Dark => rgb8(0x0f, 0x6c, 0xbf),
            Self::Dark2 => rgb8(0x00, 0x99, 0xbc),
        }
    }

    /// The background color of the tab strip.
    #[must_use]
    pub fn tab_bar_background(self) -> Color {
        match self {
            Self::Office2013 => rgb8(0xff, 0xff, 0xff),
            Self::Office2016Blue | Self::Office2016Green => rgb8(0xff, 0xff, 0xff),
            Self::Office2016Dark => rgb8(0x1e, 0x1e, 0x1e),
            Self::Office2021Blue | Self::Office2021Green => rgb8(0xfa, 0xfa, 0xfa),
            Self::Office2021Dark => rgb8(0x25, 0x25, 0x25),
            Self::Windows7 => rgb8(0xe8, 0xf0, 0xf8),
            Self::Dark => rgb8(0x1e, 0x1e, 0x1e),
            Self::Dark2 => rgb8(0x1e, 0x1e, 0x1e),
        }
    }

    /// The background color of the group area (beneath the tabs).
    #[must_use]
    pub fn group_background(self) -> Color {
        match self {
            Self::Office2013 => rgb8(0xf0, 0xf0, 0xf0),
            Self::Office2016Blue | Self::Office2016Green => rgb8(0xf5, 0xf5, 0xf5),
            Self::Office2016Dark => rgb8(0x2d, 0x2d, 0x2d),
            Self::Office2021Blue | Self::Office2021Green => rgb8(0xf0, 0xf0, 0xf0),
            Self::Office2021Dark => rgb8(0x32, 0x32, 0x32),
            Self::Windows7 => rgb8(0xdd, 0xe8, 0xf5),
            Self::Dark => rgb8(0x2d, 0x2d, 0x2d),
            Self::Dark2 => rgb8(0x2d, 0x2d, 0x2d),
        }
    }

    /// The text color for tab labels.
    #[must_use]
    pub fn tab_text(self) -> Color {
        match self {
            Self::Office2013
            | Self::Office2016Blue
            | Self::Office2016Green
            | Self::Office2021Blue
            | Self::Office2021Green
            | Self::Windows7 => rgb8(0x33, 0x33, 0x33),
            Self::Office2016Dark | Self::Office2021Dark | Self::Dark | Self::Dark2 => {
                rgb8(0xe0, 0xe0, 0xe0)
            }
        }
    }

    /// The text color for the active tab label.
    #[must_use]
    pub fn tab_active_text(self) -> Color {
        self.accent()
    }

    /// The hover background for a tool button.
    #[must_use]
    pub fn button_hover(self) -> Color {
        match self {
            Self::Office2013 => rgb8(0xe5, 0xf1, 0xfb),
            Self::Office2016Blue => rgb8(0xe8, 0xf0, 0xfe),
            Self::Office2016Green => rgb8(0xe6, 0xf4, 0xea),
            Self::Office2016Dark => rgb8(0x3c, 0x3c, 0x3c),
            Self::Office2021Blue => rgb8(0xe8, 0xf0, 0xfe),
            Self::Office2021Green => rgb8(0xe6, 0xf4, 0xea),
            Self::Office2021Dark => rgb8(0x3c, 0x3c, 0x3c),
            Self::Windows7 => rgb8(0xc8, 0xdc, 0xf0),
            Self::Dark => rgb8(0x3c, 0x3c, 0x3c),
            Self::Dark2 => rgb8(0x3c, 0x3c, 0x3c),
        }
    }

    /// The border color for the ribbon band.
    #[must_use]
    pub fn border(self) -> Color {
        match self {
            Self::Office2013 => rgb8(0xd0, 0xd0, 0xd0),
            Self::Office2016Blue | Self::Office2016Green => rgb8(0xe0, 0xe0, 0xe0),
            Self::Office2016Dark => rgb8(0x40, 0x40, 0x40),
            Self::Office2021Blue | Self::Office2021Green => rgb8(0xe0, 0xe0, 0xe0),
            Self::Office2021Dark => rgb8(0x40, 0x40, 0x40),
            Self::Windows7 => rgb8(0xb0, 0xc8, 0xe0),
            Self::Dark => rgb8(0x40, 0x40, 0x40),
            Self::Dark2 => rgb8(0x40, 0x40, 0x40),
        }
    }

    /// The text color for tool labels.
    #[must_use]
    pub fn tool_text(self) -> Color {
        match self {
            Self::Office2013
            | Self::Office2016Blue
            | Self::Office2016Green
            | Self::Office2021Blue
            | Self::Office2021Green
            | Self::Windows7 => rgb8(0x33, 0x33, 0x33),
            Self::Office2016Dark | Self::Office2021Dark | Self::Dark | Self::Dark2 => {
                rgb8(0xe0, 0xe0, 0xe0)
            }
        }
    }

    /// The muted foreground color (group labels, etc.).
    #[must_use]
    pub fn muted_foreground(self) -> Color {
        match self {
            Self::Office2013
            | Self::Office2016Blue
            | Self::Office2016Green
            | Self::Office2021Blue
            | Self::Office2021Green
            | Self::Windows7 => rgb8(0x80, 0x80, 0x80),
            Self::Office2016Dark | Self::Office2021Dark | Self::Dark | Self::Dark2 => {
                rgb8(0x90, 0x90, 0x90)
            }
        }
    }

    /// Whether this is a dark theme (background is dark).
    #[must_use]
    pub fn is_dark(self) -> bool {
        matches!(
            self,
            Self::Office2016Dark | Self::Office2021Dark | Self::Dark | Self::Dark2
        )
    }
}

impl std::fmt::Display for RibbonTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Builds an opaque [`Color`] from 8-bit sRGB components.
const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::RibbonTheme;

    #[test]
    fn all_themes_have_labels() {
        for theme in RibbonTheme::ALL {
            assert!(!theme.label().is_empty());
        }
    }

    #[test]
    fn accent_colors_are_opaque() {
        for theme in RibbonTheme::ALL {
            assert_eq!(theme.accent().a, 1.0);
        }
    }

    #[test]
    fn dark_themes_report_dark() {
        assert!(!RibbonTheme::Office2013.is_dark());
        assert!(!RibbonTheme::Office2016Blue.is_dark());
        assert!(RibbonTheme::Office2016Dark.is_dark());
        assert!(RibbonTheme::Dark.is_dark());
        assert!(RibbonTheme::Dark2.is_dark());
    }
}

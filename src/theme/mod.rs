//! The `iced-kit` theme.
//!
//! iced 0.14 widgets are generic over their theme type, so `iced-kit` ships its
//! own [`Theme`] carrying the full semantic token set. Implementing the
//! upstream [`Catalog`] traits for it means both `iced-kit` components *and*
//! stock `iced` widgets render with the same design language.
//!
//! [`Catalog`]: iced::widget::button::Catalog

mod tokens;

pub mod catalog;

pub use tokens::{Colors, Radius, Size, Spacing, TextStyle, Tokens, Typography};

use iced::theme::{self, Mode, Palette};
use iced::{Color, Font};

/// The `iced-kit` theme: a semantic token set plus its resolved mode.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    tokens: Tokens,
    mode: Mode,
}

impl Theme {
    /// The default light theme.
    #[must_use]
    pub fn light() -> Self {
        Self {
            tokens: Tokens::light(),
            mode: Mode::Light,
        }
    }

    /// The default dark theme.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            tokens: Tokens::dark(),
            mode: Mode::Dark,
        }
    }

    /// Builds a theme from an explicit token set.
    ///
    /// The [`Mode`] is inferred from the background lightness so that the iced
    /// runtime picks matching built-in UI (window chrome, devtools).
    #[must_use]
    pub fn from_tokens(tokens: Tokens) -> Self {
        let mode = if luminance(tokens.colors.background) < 0.5 {
            Mode::Dark
        } else {
            Mode::Light
        };

        Self { tokens, mode }
    }

    /// Returns this theme's semantic tokens.
    #[must_use]
    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// Returns this theme's color roles.
    #[must_use]
    pub fn colors(&self) -> &Colors {
        &self.tokens.colors
    }

    /// Returns this theme's radius scale.
    #[must_use]
    pub fn radius(&self) -> &Radius {
        &self.tokens.radius
    }

    /// Returns this theme's spacing scale.
    #[must_use]
    pub fn spacing(&self) -> &Spacing {
        &self.tokens.spacing
    }

    /// Returns this theme's type scale and font families.
    #[must_use]
    pub fn typography(&self) -> &Typography {
        &self.tokens.typography
    }

    /// Whether this theme is a dark theme.
    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.mode == Mode::Dark
    }

    /// Returns the theme with different tokens, keeping the same mode.
    #[must_use]
    pub fn with_tokens(mut self, tokens: Tokens) -> Self {
        self.mode = if luminance(tokens.colors.background) < 0.5 {
            Mode::Dark
        } else {
            Mode::Light
        };
        self.tokens = tokens;
        self
    }

    /// Returns a copy of this theme with a different primary color.
    #[must_use]
    pub fn with_primary(mut self, primary: Color) -> Self {
        self.tokens.colors.primary = primary;
        self
    }

    /// Returns a copy of this theme with a different font family.
    #[must_use]
    pub fn with_font(mut self, font: Font) -> Self {
        self.tokens.typography.sans = font;
        self
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl theme::Base for Theme {
    fn default(preference: Mode) -> Self {
        match preference {
            Mode::Dark => Self::dark(),
            // `Mode::None` means "the system expressed no preference", and
            // light is the safer default for a design system.
            Mode::Light | Mode::None => Self::light(),
        }
    }

    fn mode(&self) -> Mode {
        self.mode
    }

    fn base(&self) -> theme::Style {
        theme::Style {
            background_color: self.tokens.colors.background,
            text_color: self.tokens.colors.foreground,
        }
    }

    fn palette(&self) -> Option<Palette> {
        let colors = &self.tokens.colors;

        Some(Palette {
            background: colors.background,
            text: colors.foreground,
            primary: colors.primary,
            // The gpui-kit token set models a single `destructive` role and has
            // no success/warning roles; the iced runtime only uses these two for
            // devtools, so they map onto the closest existing roles.
            success: colors.primary,
            warning: colors.destructive,
            danger: colors.destructive,
        })
    }

    fn name(&self) -> &str {
        match self.mode {
            Mode::Dark => "iced-kit Dark",
            Mode::Light => "iced-kit Light",
            Mode::None => "iced-kit",
        }
    }
}

/// Perceptual lightness of a color, used to infer a theme's [`Mode`].
fn luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

#[cfg(test)]
mod tests {
    use super::{luminance, Theme};
    use iced::theme::{Base, Mode};

    #[test]
    fn light_and_dark_report_their_mode() {
        assert_eq!(Theme::light().mode(), Mode::Light);
        assert_eq!(Theme::dark().mode(), Mode::Dark);
        assert!(Theme::dark().is_dark());
        assert!(!Theme::light().is_dark());
    }

    #[test]
    fn default_prefers_light_when_the_system_is_silent() {
        let from_mode = |mode| <Theme as Base>::default(mode).mode();

        assert_eq!(from_mode(Mode::None), Mode::Light);
        assert_eq!(from_mode(Mode::Dark), Mode::Dark);
        assert_eq!(from_mode(Mode::Light), Mode::Light);
    }

    #[test]
    fn from_tokens_infers_the_mode_from_the_background() {
        let mut tokens = crate::theme::Tokens::light();
        tokens.colors.background = iced::Color::from_rgb8(0x0a, 0x0a, 0x0a);
        assert_eq!(Theme::from_tokens(tokens).mode(), Mode::Dark);
    }

    #[test]
    fn base_style_exposes_the_semantic_colors() {
        let theme = Theme::dark();
        let base = theme.base();
        assert_eq!(base.background_color, theme.colors().background);
        assert_eq!(base.text_color, theme.colors().foreground);
    }

    #[test]
    fn palette_bridges_to_the_iced_runtime() {
        let theme = Theme::light();
        let palette = theme.palette().expect("a palette is always available");
        assert_eq!(palette.background, theme.colors().background);
        assert_eq!(palette.primary, theme.colors().primary);
    }

    #[test]
    fn luminance_separates_the_two_palettes() {
        let light = Theme::light();
        let dark = Theme::dark();
        assert!(luminance(light.colors().background) > 0.5);
        assert!(luminance(dark.colors().background) < 0.5);
    }

    #[test]
    fn theme_is_named_for_its_mode() {
        assert_eq!(Theme::light().name(), "iced-kit Light");
        assert_eq!(Theme::dark().name(), "iced-kit Dark");
    }
}

//! Semantic design tokens.
//!
//! These tokens describe visual *roles* rather than components, mirroring the
//! token model of `gpui-kit`'s `SemanticThemeTokens`. Nothing here mentions a
//! `button` or a `table`: a component asks for `colors.primary`, not for "the
//! button color".

use iced::{Color, Font};

/// The full semantic token set of a theme.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tokens {
    pub colors: Colors,
    pub radius: Radius,
    pub spacing: Spacing,
    pub typography: Typography,
}

impl Tokens {
    /// The default light token set.
    #[must_use]
    pub fn light() -> Self {
        Self {
            colors: Colors::light(),
            ..Self::default()
        }
    }

    /// The default dark token set.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            colors: Colors::dark(),
            ..Self::default()
        }
    }
}

/// Semantic color roles, following the shadcn/ui naming that `gpui-kit` uses.
///
/// Every `*_foreground` color is meant to be legible on top of the color it is
/// named after.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    /// The page background.
    pub background: Color,
    /// The default text color on [`Colors::background`].
    pub foreground: Color,
    /// The background of raised surfaces such as cards and popovers.
    pub surface: Color,
    /// The default text color on [`Colors::surface`].
    pub surface_foreground: Color,
    /// The brand color, used for primary actions.
    pub primary: Color,
    /// The text color on [`Colors::primary`].
    pub primary_foreground: Color,
    /// A lower-emphasis action color.
    pub secondary: Color,
    /// The text color on [`Colors::secondary`].
    pub secondary_foreground: Color,
    /// A de-emphasized background for disabled and inert areas.
    pub muted: Color,
    /// The text color on [`Colors::muted`].
    pub muted_foreground: Color,
    /// A subtle highlight, used for hovered rows and selected items.
    pub accent: Color,
    /// The text color on [`Colors::accent`].
    pub accent_foreground: Color,
    /// A cautionary color, for warnings and risky actions.
    pub warning: Color,
    /// The text color on [`Colors::warning`].
    pub warning_foreground: Color,
    /// A positive color, for confirmations and completed work.
    pub success: Color,
    /// The text color on [`Colors::success`].
    pub success_foreground: Color,
    /// An informational color, for neutral notices.
    pub info: Color,
    /// The text color on [`Colors::info`].
    pub info_foreground: Color,
    /// The color of destructive actions.
    pub destructive: Color,
    /// The text color on [`Colors::destructive`].
    pub destructive_foreground: Color,
    /// The color of hyperlinks.
    pub link: Color,
    /// The text color of a hovered hyperlink.
    pub link_hover: Color,
    /// The color of borders between elements.
    pub border: Color,
    /// The border color of form controls.
    pub input: Color,
    /// The focus ring color.
    pub ring: Color,
    /// The background painted behind selected text.
    pub selection: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self::light()
    }
}

impl Colors {
    /// The default light palette, aligned with `gpui-kit`'s Default Light theme.
    ///
    /// `gpui-kit` expresses these tokens in HSL; the values here are the same
    /// colors converted to sRGB.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            background: rgb(0xff, 0xff, 0xff),
            foreground: rgb(0x0a, 0x0a, 0x0a),
            surface: rgb(0xff, 0xff, 0xff),
            surface_foreground: rgb(0x0a, 0x0a, 0x0a),
            primary: rgb(0x17, 0x17, 0x17),
            primary_foreground: rgb(0xfa, 0xfa, 0xfa),
            secondary: rgb(0xe5, 0xe5, 0xe5),
            secondary_foreground: rgb(0x17, 0x17, 0x17),
            muted: rgb(0xf5, 0xf5, 0xf5),
            muted_foreground: rgb(0x73, 0x73, 0x73),
            accent: rgb(0xf5, 0xf5, 0xf5),
            accent_foreground: rgb(0x17, 0x17, 0x17),
            destructive: rgb(0xef, 0x44, 0x44),
            destructive_foreground: rgb(0xfa, 0xfa, 0xfa),
            warning: rgb(0xf5, 0x9e, 0x0b),
            warning_foreground: rgb(0x17, 0x17, 0x17),
            success: rgb(0x22, 0xc5, 0x5e),
            success_foreground: rgb(0x17, 0x17, 0x17),
            info: rgb(0x06, 0xb6, 0xd4),
            info_foreground: rgb(0x17, 0x17, 0x17),
            link: rgb(0x0a, 0x0a, 0x0a),
            link_hover: rgb(0x40, 0x40, 0x40),
            border: rgb(0xe5, 0xe5, 0xe5),
            input: rgb(0xe5, 0xe5, 0xe5),
            ring: rgb(0xa3, 0xa3, 0xa3),
            selection: Color::from_rgba(
                0x55 as f32 / 255.0,
                0xa0 as f32 / 255.0,
                0xfc as f32 / 255.0,
                0.3,
            ),
        }
    }

    /// The default dark palette, aligned with `gpui-kit`'s Default Dark theme.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            background: rgb(0x0a, 0x0a, 0x0a),
            foreground: rgb(0xfa, 0xfa, 0xfa),
            surface: rgb(0x0a, 0x0a, 0x0a),
            surface_foreground: rgb(0xfa, 0xfa, 0xfa),
            primary: rgb(0xfa, 0xfa, 0xfa),
            primary_foreground: rgb(0x17, 0x17, 0x17),
            secondary: rgb(0x26, 0x26, 0x26),
            secondary_foreground: rgb(0xfa, 0xfa, 0xfa),
            muted: rgb(0x26, 0x26, 0x26),
            muted_foreground: rgb(0xa3, 0xa3, 0xa3),
            accent: rgb(0x26, 0x26, 0x26),
            accent_foreground: rgb(0xfa, 0xfa, 0xfa),
            destructive: rgb(0xd4, 0x2c, 0x2c),
            destructive_foreground: rgb(0xfa, 0xfa, 0xfa),
            warning: rgb(0xfb, 0xbf, 0x24),
            warning_foreground: rgb(0x0a, 0x0a, 0x0a),
            success: rgb(0x4a, 0xde, 0x80),
            success_foreground: rgb(0x0a, 0x0a, 0x0a),
            info: rgb(0x22, 0xd3, 0xee),
            info_foreground: rgb(0x0a, 0x0a, 0x0a),
            link: rgb(0xfa, 0xfa, 0xfa),
            link_hover: rgb(0xff, 0xff, 0xff),
            border: rgb(0x26, 0x26, 0x26),
            input: rgb(0x2e, 0x2e, 0x2e),
            ring: rgb(0x73, 0x73, 0x73),
            selection: Color::from_rgba(
                0x1d as f32 / 255.0,
                0x4e as f32 / 255.0,
                0xd8 as f32 / 255.0,
                0.3,
            ),
        }
    }
}

/// Corner radii.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radius {
    pub none: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
    pub full: u16,
}

impl Radius {
    /// The medium step of the default scale.
    ///
    /// A widget that has to know its radius before it can be given a theme —
    /// a frame resolved outside the draw pass, say — uses this. Every palette
    /// in this crate shares one radius scale, so the default is the right value,
    /// and naming it here keeps the two from drifting apart.
    pub const DEFAULT_MD: u16 = 6;
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            none: 0,
            sm: 3,
            md: 6,
            lg: 8,
            xl: 12,
            full: u16::MAX,
        }
    }
}

/// Spacing steps, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing {
    pub xxs: u16,
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
    pub xxl: u16,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xxs: 2,
            xs: 4,
            sm: 8,
            md: 12,
            lg: 16,
            xl: 24,
            xxl: 32,
        }
    }
}

/// One step of the type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub line_height: f32,
}

impl TextStyle {
    /// The line height as a widget-ready value.
    ///
    /// iced's `From<f32> for LineHeight` treats a bare `f32` as a *relative*
    /// multiple of the font size, so passing this token straight into
    /// `.line_height(...)` would scale lines by the pixel count. The value has
    /// to be wrapped in `Pixels` to mean what the token says.
    #[must_use]
    pub fn line_height(self) -> iced::Pixels {
        iced::Pixels(self.line_height)
    }
}

/// The type scale and font families.
#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    pub sans: Font,
    pub mono: Font,
    pub xs: TextStyle,
    pub sm: TextStyle,
    pub md: TextStyle,
    pub lg: TextStyle,
    pub xl: TextStyle,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            sans: Font::DEFAULT,
            mono: Font::MONOSPACE,
            xs: TextStyle {
                size: 12.0,
                line_height: 16.0,
            },
            sm: TextStyle {
                size: 14.0,
                line_height: 20.0,
            },
            md: TextStyle {
                size: 16.0,
                line_height: 24.0,
            },
            lg: TextStyle {
                size: 18.0,
                line_height: 28.0,
            },
            xl: TextStyle {
                size: 20.0,
                line_height: 28.0,
            },
        }
    }
}

/// The size of a control along its dominant axis.
///
/// The steps mirror `gpui-kit`'s `Size`, including its escape hatch for an
/// explicit pixel size. That variant is why the type is `PartialEq` rather
/// than `Eq`: a `f32` cannot promise the reflexivity `Eq` requires.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Size {
    /// The most compact control, for dense toolbars and table rows.
    Xs,
    /// A compact control.
    Sm,
    /// The default control size.
    #[default]
    Md,
    /// A prominent control, for primary calls to action.
    Lg,
    /// An explicit size in logical pixels, bypassing the scale.
    Custom(f32),
}

impl Size {
    /// The height of a control of this size.
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Xs => 20.0,
            Self::Sm => 28.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
            Self::Custom(height) => height,
        }
    }

    /// The horizontal padding of a control of this size.
    #[must_use]
    pub const fn padding(self) -> f32 {
        match self {
            Self::Xs => 6.0,
            Self::Sm => 8.0,
            Self::Md => 12.0,
            Self::Lg => 16.0,
            Self::Custom(padding) => padding,
        }
    }

    /// The type scale step a control of this size uses.
    #[must_use]
    pub const fn text(self) -> TextStyle {
        match self {
            Self::Xs => TextStyle {
                size: 12.0,
                line_height: 16.0,
            },
            Self::Sm => TextStyle {
                size: 13.0,
                line_height: 18.0,
            },
            Self::Md => TextStyle {
                size: 14.0,
                line_height: 20.0,
            },
            Self::Lg => TextStyle {
                size: 15.0,
                line_height: 22.0,
            },
            // An explicit size carries no typographic intent of its own, so it
            // borrows the default step.
            Self::Custom(_) => Self::Md.text(),
        }
    }

    /// The gap between an icon and its label at this size.
    ///
    /// A compact control needs a tighter gap, or the icon and label read as two
    /// separate controls.
    #[must_use]
    pub const fn gap(self) -> f32 {
        match self {
            Self::Xs | Self::Sm => 4.0,
            Self::Md | Self::Lg | Self::Custom(_) => 8.0,
        }
    }

    /// The side of a square icon-only control of this size.
    ///
    /// An icon-only control is as tall as a labelled one but only as wide as it
    /// is tall, which is what makes it read as a button rather than a gap.
    #[must_use]
    pub const fn icon_side(self) -> f32 {
        self.height()
    }

    /// The diameter of an icon drawn at this size.
    ///
    /// An icon is set smaller than its control so the glyph does not crowd the
    /// border, following upstream's three-quarter ratio.
    #[must_use]
    pub const fn icon_size(self) -> f32 {
        match self {
            Self::Xs => 12.0,
            Self::Sm => 14.0,
            Self::Md => 16.0,
            Self::Lg => 18.0,
            Self::Custom(size) => size * 0.5,
        }
    }

    /// The horizontal inset between a form control's border and its content.
    ///
    /// A field sets its text closer to the border than a button sets its label:
    /// the value is what the eye tracks, and the shorter run to the border is
    /// what makes the box read as a place to type. This is `gpui-kit`'s
    /// `input_px`.
    #[must_use]
    pub const fn input_padding(self) -> f32 {
        match self {
            Self::Xs => 4.0,
            Self::Sm => 8.0,
            Self::Lg => 12.0,
            // An explicit size carries no inset of its own, so it borrows the
            // default step's.
            Self::Md | Self::Custom(_) => 10.0,
        }
    }

    /// The height of a form control's content box, excluding its border.
    ///
    /// A field's border is one pixel on each side and the text is centred in
    /// what remains, so this is the value a field's line box has to equal.
    #[must_use]
    pub const fn input_inner_height(self) -> f32 {
        self.height() - 2.0
    }
}

impl From<f32> for Size {
    fn from(height: f32) -> Self {
        Self::Custom(height)
    }
}

impl From<u16> for Size {
    fn from(height: u16) -> Self {
        Self::Custom(f32::from(height))
    }
}

/// Builds an opaque [`Color`] from 8-bit sRGB components.
const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::{Colors, Radius, Size, TextStyle, Tokens};

    #[test]
    fn default_tokens_are_the_light_palette() {
        assert_eq!(Tokens::default().colors, Colors::light());
    }

    #[test]
    fn palettes_are_opaque_and_contrasting() {
        let light = Colors::light();
        let dark = Colors::dark();

        // Foreground must sit on the opposite end of the lightness range from
        // the background it is drawn on, in both palettes.
        let luminance = |c: iced::Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;

        assert!(luminance(light.foreground) < luminance(light.background));
        assert!(luminance(dark.foreground) > luminance(dark.background));

        for color in [
            light.primary,
            dark.primary,
            light.destructive,
            dark.destructive,
        ] {
            assert_eq!(color.a, 1.0, "semantic colors must be opaque");
        }
    }

    #[test]
    fn sizes_are_ordered_and_text_stays_within_them() {
        assert!(Size::Xs.height() < Size::Sm.height());
        assert!(Size::Sm.height() < Size::Md.height());
        assert!(Size::Md.height() < Size::Lg.height());

        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let text = size.text();
            assert!(
                text.line_height <= size.height(),
                "a control must be tall enough for its own text"
            );
        }
    }

    /// An explicit size must bypass the scale entirely, and must not inherit a
    /// type step proportional to its pixel count — a 200px button showing 100px
    /// text would be nonsense.
    #[test]
    fn a_custom_size_carries_its_own_geometry_only() {
        let custom = Size::Custom(44.0);

        assert_eq!(custom.height(), 44.0);
        assert_eq!(custom.text(), Size::Md.text());
        assert_eq!(custom.icon_size(), 22.0);
    }

    /// The icon gap narrows on compact controls; that is what keeps an icon and
    /// its label reading as one control rather than two.
    #[test]
    fn compact_sizes_use_a_tighter_icon_gap() {
        assert!(Size::Sm.gap() < Size::Md.gap());
        assert_eq!(Size::Xs.gap(), Size::Sm.gap());
    }

    /// iced reads a bare `f32` line height as a multiple of the font size, so a
    /// token must be passed through [`TextStyle::line_height`] to mean pixels.
    /// Getting this wrong inflates every line to hundreds of pixels and pushes
    /// button labels out of their own buttons, so the regression is pinned here.
    #[test]
    fn line_height_tokens_are_absolute_and_fit_their_text_size() {
        const TEXT_STEPS: [TextStyle; 3] = [
            TextStyle {
                size: 12.0,
                line_height: 16.0,
            },
            TextStyle {
                size: 14.0,
                line_height: 20.0,
            },
            TextStyle {
                size: 20.0,
                line_height: 28.0,
            },
        ];

        for step in TEXT_STEPS {
            let absolute: iced::Pixels = step.line_height();
            assert_eq!(
                absolute.0, step.line_height,
                "the token must be handed to iced as absolute pixels"
            );
            assert!(
                absolute.0 >= step.size,
                "a line must be at least as tall as its text"
            );
            assert!(
                absolute.0 < step.size * 4.0,
                "a line height near a multiple of the font size means the value \
                 was read as relative, not absolute"
            );
        }
    }

    #[test]
    fn radius_scale_is_monotonic() {
        let radius = Radius::default();
        assert!(radius.sm < radius.md);
        assert!(radius.md < radius.lg);
        assert!(radius.lg < radius.xl);
    }
}

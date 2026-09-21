//! Catalog implementations bridging [`Theme`] into iced's widgets.
//!
//! Every styled iced widget looks up its appearance through a `Catalog` trait.
//! Implementing those traits for [`Theme`] is what lets an application mix stock
//! iced widgets (`button`, `text_input`, `checkbox`, …) with `iced-kit`
//! components and still get one consistent look.
//!
//! The functions in this module are the reusable styling primitives; the
//! components in [`crate::widgets`] build on the same tokens.

use crate::theme::{Size, Theme};
use iced::widget::overlay::menu;
use iced::widget::{
    button, checkbox, container, markdown, pane_grid, pick_list, progress_bar, radio, rule,
    scrollable, slider, svg, table, text, text_editor, text_input, toggler,
};
use iced::{Background, Border, Color, Shadow, Vector};

/// Multiplies a color's alpha, for disabled and de-emphasized states.
fn fade(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}

/// Lightens (positive `amount`) or darkens (negative `amount`) a color.
fn shade(color: Color, amount: f32) -> Color {
    if amount >= 0.0 {
        Color {
            r: color.r + (1.0 - color.r) * amount,
            g: color.g + (1.0 - color.g) * amount,
            b: color.b + (1.0 - color.b) * amount,
            a: color.a,
        }
    } else {
        let scale = 1.0 + amount;
        Color {
            r: color.r * scale,
            g: color.g * scale,
            b: color.b * scale,
            a: color.a,
        }
    }
}

/// Blends `top` over `bottom` using `top`'s alpha.
///
/// iced colors carry their own alpha, so a translucent color that has to sit on
/// a known surface (a border over a tinted panel, say) must be composited by
/// hand before it is handed to a style struct.
pub fn blend(top: Color, bottom: Color) -> Color {
    let a = top.a;
    Color {
        r: top.r * a + bottom.r * (1.0 - a),
        g: top.g * a + bottom.g * (1.0 - a),
        b: top.b * a + bottom.b * (1.0 - a),
        a: 1.0,
    }
}

/// Picks the foreground that stays legible on the given background.
fn readable_on(background: Color) -> Color {
    let luminance = 0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;

    if luminance > 0.5 {
        Color::from_rgb8(0x0a, 0x0a, 0x0a)
    } else {
        Color::from_rgb8(0xfa, 0xfa, 0xfa)
    }
}

/// Scales a color's alpha.
///
/// This reproduces `gpui-kit`'s `mix_oklab(color, transparent, factor)` for the
/// case its button tokens use: mixing with a fully transparent color in a
/// premultiplied space leaves hue and lightness alone and scales alpha, so the
/// result is exactly the input color at `factor` strength.
fn tint(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}

/// Washes a color toward white, which is how upstream derives an outline
/// button's border from its accent.
fn soften(color: Color, factor: f32) -> Color {
    blend(tint(color, factor), Color::WHITE)
}

/// The surface a form control rests on.
///
/// In dark mode a control has to sit slightly above the page to be visible, so
/// it is a faint wash of the input border; in light mode the page itself is
/// already the right surface and a fill would only add noise.
///
/// This is `gpui-kit`'s `input_background`.
#[must_use]
pub fn field_surface(theme: &Theme) -> Color {
    let colors = theme.colors();

    if theme.is_dark() {
        tint(colors.input, 0.3)
    } else {
        colors.background
    }
}

/// Which corners of a control are rounded.
///
/// A button on its own rounds all four. Inside a group only the outer corners
/// are rounded, which is what makes the members read as one joined control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Corners {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_right: bool,
    pub bottom_left: bool,
}

impl Corners {
    /// Every corner rounded, the shape of a standalone control.
    pub const ALL: Self = Self {
        top_left: true,
        top_right: true,
        bottom_right: true,
        bottom_left: true,
    };

    /// No corner rounded.
    pub const NONE: Self = Self {
        top_left: false,
        top_right: false,
        bottom_right: false,
        bottom_left: false,
    };

    /// Applies this mask to a radius, zeroing the corners it excludes.
    #[must_use]
    pub fn apply(self, radius: ButtonRounded, theme: &Theme) -> iced::border::Radius {
        let resolved = radius.resolve(theme);
        iced::border::Radius {
            top_left: if self.top_left { resolved } else { 0.0 },
            top_right: if self.top_right { resolved } else { 0.0 },
            bottom_right: if self.bottom_right { resolved } else { 0.0 },
            bottom_left: if self.bottom_left { resolved } else { 0.0 },
        }
    }
}

impl Default for Corners {
    fn default() -> Self {
        Self::ALL
    }
}

/// How round a button is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonRounded {
    /// Square corners.
    None,
    /// Half the theme radius.
    Small,
    /// The theme radius.
    #[default]
    Medium,
    /// Twice the theme radius.
    Large,
    /// An explicit radius in logical pixels.
    Custom(f32),
}

impl ButtonRounded {
    /// Resolves this choice against the theme's radius scale.
    #[must_use]
    pub fn resolve(self, theme: &Theme) -> f32 {
        let base = f32::from(theme.radius().md);

        match self {
            Self::None => 0.0,
            Self::Small => base * 0.5,
            Self::Medium => base,
            Self::Large => base * 2.0,
            Self::Custom(radius) => radius,
        }
    }
}

impl From<f32> for ButtonRounded {
    fn from(radius: f32) -> Self {
        Self::Custom(radius)
    }
}

/// A caller-supplied button palette.
///
/// Use this when none of the named variants fits — a brand color that is not
/// the theme's `primary`, say. The three interaction colors default to fully
/// transparent, so a custom variant with only `color` set does not react to the
/// pointer; set `hover` and `active` to opt into feedback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonCustomVariant {
    /// The resting background.
    pub color: Color,
    /// The text and icon color.
    pub foreground: Color,
    /// The background while hovered.
    pub hover: Color,
    /// The background while pressed.
    pub active: Color,
    /// Whether the button casts a small shadow.
    pub shadow: bool,
}

impl ButtonCustomVariant {
    /// A custom variant with the given background.
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            foreground: readable_on(color),
            hover: Color::TRANSPARENT,
            active: Color::TRANSPARENT,
            shadow: false,
        }
    }

    /// Sets the text and icon color.
    #[must_use]
    pub fn foreground(mut self, foreground: Color) -> Self {
        self.foreground = foreground;
        self
    }

    /// Sets the hovered background.
    #[must_use]
    pub fn hover(mut self, hover: Color) -> Self {
        self.hover = hover;
        self
    }

    /// Sets the pressed background.
    #[must_use]
    pub fn active(mut self, active: Color) -> Self {
        self.active = active;
        self
    }

    /// Sets whether the button casts a small shadow.
    #[must_use]
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

/// The variants a button can take, mirroring `gpui-kit`'s `ButtonVariant`.
///
/// This is not `Eq` because [`ButtonVariant::Custom`] carries colors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    /// A neutral button: a surface fill with an input-colored border.
    #[default]
    Default,
    /// The main call to action, filled with the brand color.
    Primary,
    /// A lower-emphasis filled button.
    Secondary,
    /// A soft destructive action: a red wash with red text.
    Danger,
    /// A cautionary action.
    Warning,
    /// A confirming action.
    Success,
    /// An informational action.
    Info,
    /// A transparent button that reacts on hover.
    Ghost,
    /// A button that reads as a hyperlink, underlined in every state.
    Link,
    /// A button that reads as plain text, with no padding or border.
    Text,
    /// A filled destructive action.
    ///
    /// This is an `iced-kit` addition: upstream's [`Danger`](Self::Danger) is a
    /// soft tint, which is the wrong signal for a genuinely irreversible
    /// action such as a delete.
    Destructive,
    /// A caller-supplied palette.
    Custom(ButtonCustomVariant),
}

impl ButtonVariant {
    /// Whether this variant draws no padding, so it reads as inline content.
    #[must_use]
    pub fn no_padding(self) -> bool {
        matches!(self, Self::Link | Self::Text)
    }

    /// Whether this variant is drawn as a link.
    #[must_use]
    pub fn is_link(self) -> bool {
        matches!(self, Self::Link)
    }

    /// Whether this variant draws an underline under its label in every state.
    #[must_use]
    pub fn underline(self) -> bool {
        matches!(self, Self::Link)
    }

    /// The solid accent this variant is built around, if it has one.
    fn accent(self, theme: &Theme) -> Option<Color> {
        let colors = theme.colors();

        match self {
            Self::Primary | Self::Default => Some(colors.primary),
            Self::Secondary => Some(colors.secondary),
            Self::Danger | Self::Destructive => Some(colors.destructive),
            Self::Warning => Some(colors.warning),
            Self::Success => Some(colors.success),
            Self::Info => Some(colors.info),
            Self::Custom(custom) => Some(custom.color),
            Self::Ghost | Self::Link | Self::Text => None,
        }
    }

    /// The background this variant paints outside its hovered and pressed
    /// states.
    fn resting_background(self, outline: bool, theme: &Theme) -> Option<Color> {
        let colors = theme.colors();

        // An outline button is the accent as a wash rather than a fill, which
        // is what keeps it visually behind a filled button of the same accent.
        if outline {
            return match self {
                Self::Default => Some(field_surface(theme)),
                Self::Ghost | Self::Link | Self::Text => None,
                other => other.accent(theme).map(|accent| tint(accent, 0.1)),
            };
        }

        match self {
            Self::Default => Some(field_surface(theme)),
            Self::Primary => Some(colors.primary),
            Self::Secondary => Some(colors.secondary),
            Self::Destructive => Some(colors.destructive),
            // The soft variants are a light wash of their accent, with the
            // accent itself carrying the text. That is what distinguishes
            // `Danger` from `Destructive`.
            Self::Danger => Some(tint(colors.destructive, 0.2)),
            Self::Warning => Some(tint(colors.warning, 0.2)),
            Self::Success => Some(tint(colors.success, 0.2)),
            Self::Info => Some(tint(colors.info, 0.2)),
            Self::Custom(custom) => Some(tint(custom.color, 0.2)),
            Self::Ghost | Self::Link | Self::Text => None,
        }
    }

    /// The background this variant paints while hovered.
    fn hover_background(self, outline: bool, theme: &Theme) -> Option<Color> {
        let colors = theme.colors();

        match self {
            Self::Default => Some(tint(colors.input, 0.5)),
            Self::Primary => Some(blend(tint(colors.primary, 0.9), colors.background)),
            Self::Secondary => Some(blend(tint(colors.secondary, 0.9), colors.background)),
            Self::Destructive => Some(shade(colors.destructive, -0.1)),
            Self::Danger => Some(tint(colors.destructive, 0.3)),
            Self::Warning => Some(tint(colors.warning, 0.3)),
            Self::Success => Some(tint(colors.success, 0.3)),
            Self::Info => Some(tint(colors.info, 0.3)),
            Self::Custom(custom) => Some(custom.hover),
            // A ghost button's hover surface is the accent, which upstream
            // halves in dark mode so it does not glare against a dark page.
            Self::Ghost => Some(if theme.is_dark() {
                tint(colors.accent, 0.5)
            } else {
                colors.accent
            }),
            Self::Link | Self::Text => None,
        }
        .map(|color| {
            if outline {
                match self {
                    Self::Ghost | Self::Link | Self::Text => color,
                    other => other
                        .accent(theme)
                        .map_or(color, |accent| tint(accent, 0.2)),
                }
            } else {
                color
            }
        })
    }

    /// The background this variant paints while pressed.
    fn active_background(self, outline: bool, theme: &Theme) -> Option<Color> {
        self.pressed_background(outline, theme)
    }

    /// The background a pressed or selected variant paints.
    ///
    /// The two states share a strength: a selection has to read as a stronger
    /// signal than a hover, and there is nothing above the pressed shade to
    /// escalate to.
    fn pressed_background(self, outline: bool, theme: &Theme) -> Option<Color> {
        let colors = theme.colors();

        match self {
            // A ghost button has no fill of its own, so it deepens to the same
            // shade a neutral one does.
            Self::Default | Self::Ghost => Some(tint(colors.input, 0.7)),
            Self::Primary => Some(shade(
                colors.primary,
                if theme.is_dark() { -0.2 } else { -0.1 },
            )),
            Self::Secondary => Some(shade(
                colors.secondary,
                if theme.is_dark() { -0.2 } else { -0.1 },
            )),
            Self::Destructive => Some(shade(colors.destructive, -0.2)),
            Self::Danger => Some(tint(colors.destructive, 0.4)),
            Self::Warning => Some(tint(colors.warning, 0.4)),
            Self::Success => Some(tint(colors.success, 0.4)),
            Self::Info => Some(tint(colors.info, 0.4)),
            Self::Custom(custom) => Some(custom.active),
            Self::Link | Self::Text => None,
        }
        .map(|color| {
            if outline {
                match self {
                    Self::Ghost | Self::Link | Self::Text => color,
                    other => other
                        .accent(theme)
                        .map_or(color, |accent| tint(accent, 0.4)),
                }
            } else {
                color
            }
        })
    }

    /// The background a selected or open button paints.
    ///
    /// A selection is a stronger signal than a hover, so it uses the pressed
    /// strength rather than the hovered one — otherwise a selected button would
    /// be indistinguishable from one the pointer happens to be over.
    fn selected_background(self, outline: bool, theme: &Theme) -> Option<Color> {
        let colors = theme.colors();

        if outline {
            return self.active_background(true, theme);
        }

        // A ghost button has no fill to deepen, so its selection is a surface
        // of its own rather than a stronger version of the resting one.
        if self == Self::Ghost {
            return Some(shade(
                colors.secondary,
                if theme.is_dark() { -0.2 } else { -0.1 },
            ));
        }

        self.pressed_background(false, theme)
    }

    /// The text and icon color this variant paints in a state that is not
    /// hovered or pressed.
    fn text_color(self, outline: bool, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Default => colors.foreground,
            Self::Primary => {
                if outline {
                    colors.primary
                } else {
                    colors.primary_foreground
                }
            }
            Self::Secondary => colors.secondary_foreground,
            Self::Destructive => colors.destructive_foreground,
            Self::Danger => colors.destructive,
            Self::Warning => colors.warning,
            Self::Success => colors.success,
            Self::Info => colors.info,
            Self::Ghost => colors.accent_foreground,
            Self::Link => colors.link,
            Self::Text => tint(colors.foreground, 0.9),
            Self::Custom(custom) => custom.foreground,
        }
    }

    /// The text and icon color this variant paints while hovered.
    fn hover_text_color(self, outline: bool, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Link => colors.link_hover,
            Self::Text => colors.foreground,
            _ => self.text_color(outline, theme),
        }
    }

    /// The text and icon color this variant paints while pressed.
    fn active_text_color(self, outline: bool, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Link => colors.link,
            Self::Text => tint(colors.foreground, 0.7),
            _ => self.text_color(outline, theme),
        }
    }

    /// The border color this variant draws.
    fn border_color(self, outline: bool, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Default => colors.input,
            Self::Primary => colors.primary,
            Self::Secondary => colors.border,
            Self::Destructive => colors.destructive,
            Self::Danger => {
                if outline {
                    soften(colors.destructive, 0.4)
                } else {
                    colors.destructive
                }
            }
            Self::Warning => {
                if outline {
                    soften(colors.warning, 0.4)
                } else {
                    colors.warning
                }
            }
            Self::Success => {
                if outline {
                    soften(colors.success, 0.4)
                } else {
                    colors.success
                }
            }
            Self::Info => {
                if outline {
                    soften(colors.info, 0.4)
                } else {
                    colors.info
                }
            }
            Self::Ghost | Self::Link | Self::Text => Color::TRANSPARENT,
            Self::Custom(custom) => {
                if outline {
                    soften(custom.color, 0.4)
                } else {
                    custom.color
                }
            }
        }
    }

    /// Whether this variant is built around a color of its own.
    ///
    /// A ghost, link or text button is not: it has no fill for a selection ring
    /// to sit against, so those keep their ordinary border.
    fn has_accent(self) -> bool {
        !matches!(self, Self::Ghost | Self::Link | Self::Text)
    }

    /// Whether this variant draws a border at all.
    ///
    /// Only the bordered variants do; a ghost, link or text button that drew one
    /// would stop reading as inline content.
    fn draws_border(self, outline: bool) -> bool {
        match self {
            Self::Ghost | Self::Link | Self::Text => false,
            Self::Custom(_) => outline,
            _ => true,
        }
    }
}

/// The interaction state a button is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// At rest.
    Normal,
    /// Under the pointer.
    Hovered,
    /// Being pressed.
    Active,
    /// Selected, or holding an open popup.
    Selected,
    /// Inert.
    Disabled,
}

/// The resolved appearance of a button in one state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonAppearance {
    pub background: Option<Background>,
    pub border: Border,
    pub text_color: Color,
    pub underline: bool,
    pub shadow: Shadow,
}

/// Everything a button needs to resolve its appearance.
///
/// Passing this as one value keeps the style function's signature stable as the
/// component grows, and lets a group or split button override a single aspect
/// (its corner mask, say) without reimplementing the palette.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ButtonClass {
    pub variant: ButtonVariant,
    pub size: Size,
    /// Drawn as an outline: an accent wash rather than an accent fill.
    pub outline: bool,
    /// Drawn as selected, or as the trigger of an open popup.
    pub selected: bool,
    /// Drawn as busy. A loading button keeps its normal palette, dimmed, rather
    /// than taking the disabled one: it is working, not unavailable.
    pub loading: bool,
    /// Drawn as unavailable. iced reports `Disabled` only when the button has
    /// no press handler, so a button that keeps a handler while being
    /// unavailable carries the state itself.
    pub disabled: bool,
    pub rounded: ButtonRounded,
    pub corners: Corners,
}

impl Default for ButtonClass {
    fn default() -> Self {
        Self {
            variant: ButtonVariant::default(),
            size: Size::default(),
            outline: false,
            selected: false,
            loading: false,
            disabled: false,
            rounded: ButtonRounded::default(),
            corners: Corners::ALL,
        }
    }
}

/// How far a loading button is dimmed.
///
/// Every variant dims by the same amount, which is why the fade is applied to
/// the resolved colors rather than to each token: `Ghost`, `Link` and `Text`
/// have no background to fade.
const LOADING_FADE: f32 = 0.8;

impl ButtonClass {
    /// Resolves the appearance for one state.
    ///
    /// The class's own `disabled` and `loading` flags win over `state`: iced
    /// reports `Disabled` only for a button with no press handler, so a control
    /// that keeps its handler while being unavailable has to say so here.
    #[must_use]
    pub fn appearance(&self, theme: &Theme, state: ButtonState) -> ButtonAppearance {
        if self.disabled {
            return self.disabled_appearance(theme);
        }

        // A busy button looks like itself, dimmed — in every state, including
        // the one iced reports while the pointer is over it.
        if self.loading && !self.selected {
            return fade_appearance(&self.resting(theme), LOADING_FADE);
        }

        match state {
            ButtonState::Normal => self.resting(theme),
            ButtonState::Hovered => self.hovered(theme),
            ButtonState::Active => self.active(theme),
            ButtonState::Selected => self.selected(theme),
            ButtonState::Disabled => self.disabled_appearance(theme),
        }
    }

    /// The resting appearance.
    fn resting(&self, theme: &Theme) -> ButtonAppearance {
        let variant = self.variant;
        let outline = self.outline;

        ButtonAppearance {
            background: variant
                .resting_background(outline, theme)
                .map(Background::Color),
            border: self.border(theme),
            text_color: variant.text_color(outline, theme),
            underline: variant.underline(),
            shadow: self.shadow(),
        }
    }

    fn hovered(&self, theme: &Theme) -> ButtonAppearance {
        let variant = self.variant;
        let outline = self.outline;

        ButtonAppearance {
            background: variant
                .hover_background(outline, theme)
                .map(Background::Color),
            border: self.border(theme),
            text_color: variant.hover_text_color(outline, theme),
            underline: variant.underline(),
            shadow: self.shadow(),
        }
    }

    fn active(&self, theme: &Theme) -> ButtonAppearance {
        let variant = self.variant;
        let outline = self.outline;

        ButtonAppearance {
            background: variant
                .active_background(outline, theme)
                .map(Background::Color),
            border: self.border(theme),
            text_color: variant.active_text_color(outline, theme),
            underline: variant.underline(),
            shadow: self.shadow(),
        }
    }

    fn selected(&self, theme: &Theme) -> ButtonAppearance {
        let variant = self.variant;
        let outline = self.outline;
        let mut border = self.border(theme);

        // A selected control draws a ring, because a background shift alone is
        // not enough signal: darkening an already near-black `Primary` fill is
        // invisible, so a group of primary buttons would give the user no way to
        // see which member is active. The ring sits within the existing bounds,
        // so it costs no layout.
        if variant.has_accent() {
            border.width = 2.0;
            border.color = theme.colors().ring;
        }

        ButtonAppearance {
            background: variant
                .selected_background(outline, theme)
                .map(Background::Color),
            border,
            text_color: variant.text_color(outline, theme),
            underline: variant.underline(),
            shadow: self.shadow(),
        }
    }

    /// The appearance of an inert button: it keeps its shape but loses its
    /// presence, with the fill dropped and the label faded.
    fn disabled_appearance(&self, theme: &Theme) -> ButtonAppearance {
        let variant = self.variant;
        let outline = self.outline;
        let colors = theme.colors();

        let background = if outline {
            // An outline button keeps a ghost of its wash, so the control is
            // still visibly there rather than collapsing to nothing.
            variant
                .resting_background(true, theme)
                .map(|color| tint(color, 0.5))
        } else {
            match variant {
                // These have no fill to fade at rest.
                ButtonVariant::Ghost | ButtonVariant::Link | ButtonVariant::Text => None,
                // A neutral control keeps its surface, so a disabled form still
                // reads as a form rather than as a row of labels.
                ButtonVariant::Default => Some(tint(field_surface(theme), 0.5)),
                other => other.accent(theme).map(|accent| tint(accent, 0.15)),
            }
        };

        let border = if variant == ButtonVariant::Default && !outline {
            // The hairline stays, dimmed, which is what keeps a disabled
            // button's silhouette recognizable.
            Border {
                color: fade(colors.input, 0.5),
                ..self.border(theme)
            }
        } else {
            Border {
                color: fade(self.border(theme).color, 0.5),
                ..self.border(theme)
            }
        };

        ButtonAppearance {
            background: background.map(Background::Color),
            border,
            // The muted foreground is legible on the surface in both palettes,
            // so a disabled label stays readable while clearly reading as
            // inactive.
            text_color: fade(colors.muted_foreground, 0.5),
            underline: variant.underline(),
            shadow: Shadow::default(),
        }
    }

    /// The border every interactive state draws.
    fn border(&self, theme: &Theme) -> Border {
        let variant = self.variant;
        let draws = variant.draws_border(self.outline);

        Border {
            color: variant.border_color(self.outline, theme),
            // A borderless variant keeps a transparent hairline rather than a
            // zero width, so a group member's border still lines up with its
            // neighbours'.
            width: if draws { 1.0 } else { 0.0 },
            radius: self.corners.apply(self.rounded, theme),
        }
    }

    fn shadow(&self) -> Shadow {
        match self.variant {
            ButtonVariant::Custom(custom) if custom.shadow => Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
            _ => Shadow::default(),
        }
    }

    /// The resting text color, which is what an icon beside the label has to
    /// match.
    ///
    /// iced's SVG widget reads its color from the theme rather than from the
    /// enclosing button, so an icon's color has to be resolved here and applied
    /// when the icon is built.
    #[must_use]
    pub fn icon_color(&self, theme: &Theme) -> Color {
        self.variant.text_color(self.outline, theme)
    }

    /// Converts this class into the closure iced's button widget expects.
    pub fn into_style_fn(self, theme: &Theme) -> impl Fn(&button::Status) -> button::Style + '_ {
        move |status| {
            let state = match status {
                button::Status::Active => {
                    if self.selected {
                        ButtonState::Selected
                    } else {
                        ButtonState::Normal
                    }
                }
                button::Status::Hovered => ButtonState::Hovered,
                button::Status::Pressed => ButtonState::Active,
                button::Status::Disabled => ButtonState::Disabled,
            };

            let appearance = self.appearance(theme, state);

            button::Style {
                background: appearance.background,
                text_color: appearance.text_color,
                border: appearance.border,
                shadow: appearance.shadow,
                // A button whose height is not a whole number of pixels would
                // otherwise blur its border.
                snap: true,
            }
        }
    }
}

/// Dims every color in an appearance, which is how a loading button reads as
/// busy without changing variant.
fn fade_appearance(appearance: &ButtonAppearance, factor: f32) -> ButtonAppearance {
    ButtonAppearance {
        background: appearance.background.map(|background| match background {
            Background::Color(color) => Background::Color(fade(color, factor)),
            other @ Background::Gradient(_) => other,
        }),
        border: Border {
            color: fade(appearance.border.color, factor),
            ..appearance.border
        },
        text_color: fade(appearance.text_color, factor),
        underline: appearance.underline,
        shadow: appearance.shadow,
    }
}

/// The variant a [`crate::widgets::Button`] renders as.
///
/// A convenience wrapper over [`button_class`] for the common case: no outline,
/// no selection, the theme's default rounding.
pub fn button_style(
    theme: &Theme,
    variant: ButtonVariant,
    size: Size,
) -> impl Fn(&button::Status) -> button::Style + '_ {
    button_class(
        theme,
        &ButtonClass {
            variant,
            size,
            ..ButtonClass::default()
        },
    )
}

/// Resolves a [`ButtonClass`] against a theme into a iced style function.
pub fn button_class<'a>(
    theme: &'a Theme,
    class: &ButtonClass,
) -> impl Fn(&button::Status) -> button::Style + 'a {
    let class = *class;
    move |status| class.into_style_fn(theme)(status)
}

impl button::Catalog for Theme {
    type Class<'a> = button::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let style = button_style(theme, ButtonVariant::Default, Size::Md);
            style(&status)
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        class(self, status)
    }
}

/// SVGs follow the surrounding text color, so a monochrome icon drawn from an
/// SVG source matches the label beside it.
///
/// `iced_dock`'s builder requires this catalog on the theme.
impl svg::Catalog for Theme {
    type Class<'a> = svg::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as svg::Catalog>::Class<'a> {
        Box::new(|theme, status| {
            // A hovered icon is emphasized slightly, which is what makes an
            // icon-only button feel interactive.
            let color = match status {
                svg::Status::Idle => theme.colors().foreground,
                svg::Status::Hovered => shade(theme.colors().foreground, 0.2),
            };

            svg::Style { color: Some(color) }
        })
    }

    fn style(&self, class: &<Self as svg::Catalog>::Class<'_>, status: svg::Status) -> svg::Style {
        class(self, status)
    }
}

impl container::Catalog for Theme {
    type Class<'a> = container::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(container::transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        class(self)
    }
}

/// Text always inherits its color from the theme unless a widget overrides it.
///
/// Returning `None` here is what lets a `container` or `button` set the text
/// color for everything inside it.
impl text::Catalog for Theme {
    type Class<'a> = text::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as text::Catalog>::Class<'a> {
        Box::new(|_theme| text::Style { color: None })
    }

    fn style(&self, class: &<Self as text::Catalog>::Class<'_>) -> text::Style {
        class(self)
    }
}

/// A container styled as a card: a raised surface with a border and rounded corners.
pub fn card(theme: &Theme) -> container::Style {
    let colors = theme.colors();

    container::Style {
        background: Some(Background::Color(colors.surface)),
        border: Border {
            color: colors.border,
            width: 1.0,
            radius: f32::from(theme.radius().lg).into(),
        },
        text_color: Some(colors.surface_foreground),
        ..container::Style::default()
    }
}

/// A container filled with the muted background, for inert regions.
pub fn muted(theme: &Theme) -> container::Style {
    let colors = theme.colors();

    container::Style {
        background: Some(Background::Color(colors.muted)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().md).into(),
        },
        text_color: Some(colors.muted_foreground),
        ..container::Style::default()
    }
}

/// The state a form control's frame is drawn in.
///
/// Focus, validation and the disabled state are independent of one another, so
/// they are carried as separate flags rather than as one enum: a disabled field
/// can still be showing a validation error, and the frame has to resolve that
/// combination rather than pick one. That combination is exactly why the
/// booleans are not folded into a state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct FieldState {
    /// The control inside the frame holds focus.
    pub focused: bool,
    /// The pointer is over the frame.
    pub hovered: bool,
    /// The caller's validation has failed.
    pub invalid: bool,
    /// The control is inert.
    pub disabled: bool,
}

/// The resolved appearance of a form control's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldAppearance {
    /// The fill inside the border.
    pub background: Color,
    /// The border color.
    pub border: Color,
    /// The border's width in logical pixels.
    pub border_width: f32,
    /// The color of value text.
    pub text_color: Color,
    /// The color of placeholder text, and of a prefix or suffix at rest.
    pub placeholder_color: Color,
    /// The background painted behind selected text.
    pub selection: Color,
}

impl FieldAppearance {
    /// Resolves the frame's appearance from the theme and the control's state.
    ///
    /// The precedence is deliberate, and follows `gpui-kit`'s `GroupAppearance`:
    /// an invalid field keeps its error border even while focused, because focus
    /// is not new information and replacing the error with a ring would hide the
    /// one signal the user has to act on. Focus only wins when there is no error
    /// to show. Disabled dims everything else but never cancels validation.
    #[must_use]
    pub fn resolve(theme: &Theme, state: FieldState) -> Self {
        let colors = theme.colors();

        let (border, border_width) = if state.invalid {
            (colors.destructive, 1.0)
        } else if state.focused && !state.disabled {
            // A focused field draws the ring at two pixels: the increase in
            // weight is what carries the signal, since the ring color is a
            // mid-grey in both palettes and barely differs from the input
            // border.
            (colors.ring, 2.0)
        } else if state.hovered && !state.disabled {
            (shade(colors.input, -0.15), 1.0)
        } else {
            (colors.input, 1.0)
        };

        let background = field_surface(theme);

        Self {
            background: if state.disabled {
                // A disabled field keeps a surface rather than going flat: an
                // empty box still has to read as a place a value could live.
                blend(field_surface(theme), colors.muted)
            } else {
                background
            },
            border: if state.disabled {
                fade(border, 0.5)
            } else {
                border
            },
            border_width,
            text_color: if state.disabled {
                colors.muted_foreground
            } else {
                colors.foreground
            },
            placeholder_color: colors.muted_foreground,
            selection: colors.selection,
        }
    }

    /// Converts this appearance into the closure iced's `text_input` expects.
    ///
    /// The catalog on [`Theme`] already resolves the same tokens, but it can
    /// only see the widget's *own* status — it cannot know that the caller
    /// marked the field invalid or that a group owns the frame. Passing a class
    /// built from this appearance is how those outer decisions reach the
    /// control.
    #[must_use]
    pub fn into_text_input_style(self) -> text_input::Style {
        text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            // The control sits inside a frame that draws the border, so it
            // draws none of its own; a second one would double the line.
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            icon: self.placeholder_color,
            placeholder: self.placeholder_color,
            value: self.text_color,
            selection: self.selection,
        }
    }

    /// Converts this appearance into the closure iced's `text_editor` expects.
    #[must_use]
    pub fn into_text_editor_style(self) -> text_editor::Style {
        text_editor::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            placeholder: self.placeholder_color,
            value: self.text_color,
            selection: self.selection,
        }
    }
}

impl text_input::Catalog for Theme {
    type Class<'a> = text_input::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().md);

            let (border_color, width) = match status {
                text_input::Status::Focused { .. } => (colors.ring, 2.0),
                text_input::Status::Hovered => (shade(colors.input, -0.15), 1.0),
                text_input::Status::Active | text_input::Status::Disabled => (colors.input, 1.0),
            };

            let disabled = matches!(status, text_input::Status::Disabled);

            text_input::Style {
                background: Background::Color(if disabled {
                    colors.muted
                } else {
                    colors.background
                }),
                border: Border {
                    color: border_color,
                    width,
                    radius: radius.into(),
                },
                icon: colors.muted_foreground,
                placeholder: colors.muted_foreground,
                value: if disabled {
                    colors.muted_foreground
                } else {
                    colors.foreground
                },
                selection: colors.selection,
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        class(self, status)
    }
}

impl checkbox::Catalog for Theme {
    type Class<'a> = checkbox::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().sm);
            let (checked, hovered, disabled) = match status {
                checkbox::Status::Active { is_checked } => (is_checked, false, false),
                checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
                checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
            };

            let (background, border_color) = if checked {
                (colors.primary, colors.primary)
            } else if hovered {
                (colors.accent, shade(colors.input, -0.15))
            } else {
                (colors.background, colors.input)
            };

            checkbox::Style {
                background: Background::Color(background),
                icon_color: colors.primary_foreground,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: radius.into(),
                },
                text_color: Some(if disabled {
                    colors.muted_foreground
                } else {
                    colors.foreground
                }),
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: checkbox::Status) -> checkbox::Style {
        class(self, status)
    }
}

impl radio::Catalog for Theme {
    type Class<'a> = radio::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let (selected, hovered) = match status {
                radio::Status::Active { is_selected } => (is_selected, false),
                radio::Status::Hovered { is_selected } => (is_selected, true),
            };

            radio::Style {
                background: Background::Color(colors.background),
                dot_color: colors.primary,
                border_width: 1.0,
                border_color: if selected {
                    colors.primary
                } else if hovered {
                    shade(colors.input, -0.15)
                } else {
                    colors.input
                },
                text_color: Some(colors.foreground),
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: radio::Status) -> radio::Style {
        class(self, status)
    }
}

/// The appearance of a switch, mirroring `gpui-kit`'s rounded toggler.
pub fn switch_style(theme: &Theme, size: Size) -> impl Fn(toggler::Status) -> toggler::Style + '_ {
    move |status| {
        let colors = theme.colors();
        let (toggled, hovered, disabled) = match status {
            toggler::Status::Active { is_toggled } => (is_toggled, false, false),
            toggler::Status::Hovered { is_toggled } => (is_toggled, true, false),
            toggler::Status::Disabled { is_toggled } => (is_toggled, false, true),
        };

        let track = if toggled {
            if hovered {
                shade(colors.primary, 0.1)
            } else {
                colors.primary
            }
        } else if hovered {
            shade(colors.input, -0.08)
        } else {
            colors.input
        };

        let thumb = if toggled {
            colors.primary_foreground
        } else {
            colors.background
        };

        toggler::Style {
            background: Background::Color(if disabled {
                fade(blend(track, colors.background), 0.5)
            } else {
                track
            }),
            background_border_width: 0.0,
            background_border_color: Color::TRANSPARENT,
            foreground: Background::Color(thumb),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: Some(if disabled {
                colors.muted_foreground
            } else {
                colors.foreground
            }),
            // `None` is what makes the track perfectly round, which is what
            // distinguishes a switch from a progress bar at a glance.
            border_radius: None,
            padding_ratio: match size {
                Size::Xs => 0.20,
                Size::Sm => 0.16,
                Size::Md | Size::Lg | Size::Custom(_) => 0.12,
            },
        }
    }
}

impl toggler::Catalog for Theme {
    type Class<'a> = toggler::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let style = switch_style(theme, Size::Md);
            style(status)
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: toggler::Status) -> toggler::Style {
        class(self, status)
    }
}

impl rule::Catalog for Theme {
    type Class<'a> = rule::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| rule::Style {
            color: theme.colors().border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> rule::Style {
        class(self)
    }
}

impl progress_bar::Catalog for Theme {
    type Class<'a> = progress_bar::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| {
            let colors = theme.colors();

            progress_bar::Style {
                background: Background::Color(colors.secondary),
                bar: Background::Color(colors.primary),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: f32::from(theme.radius().full.min(8)).into(),
                },
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> progress_bar::Style {
        class(self)
    }
}

impl text_editor::Catalog for Theme {
    type Class<'a> = text_editor::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().md);

            let (border_color, width) = match status {
                text_editor::Status::Focused { .. } => (colors.ring, 2.0),
                text_editor::Status::Hovered => (shade(colors.input, -0.15), 1.0),
                text_editor::Status::Active | text_editor::Status::Disabled => (colors.input, 1.0),
            };

            let disabled = matches!(status, text_editor::Status::Disabled);

            text_editor::Style {
                background: Background::Color(if disabled {
                    colors.muted
                } else {
                    colors.background
                }),
                border: Border {
                    color: border_color,
                    width,
                    radius: radius.into(),
                },
                placeholder: colors.muted_foreground,
                value: if disabled {
                    colors.muted_foreground
                } else {
                    colors.foreground
                },
                selection: colors.selection,
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: text_editor::Status) -> text_editor::Style {
        class(self, status)
    }
}

impl slider::Catalog for Theme {
    type Class<'a> = slider::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().full.min(8));
            let hovered = matches!(status, slider::Status::Hovered | slider::Status::Dragged);

            slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        Background::Color(colors.primary),
                        Background::Color(colors.secondary),
                    ),
                    width: if hovered { 6.0 } else { 4.0 },
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: radius.into(),
                    },
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle {
                        radius: if hovered { 9.0 } else { 8.0 },
                    },
                    background: Background::Color(if hovered {
                        shade(colors.primary, 0.1)
                    } else {
                        colors.primary
                    }),
                    border_width: 2.0,
                    border_color: colors.background,
                },
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: slider::Status) -> slider::Style {
        class(self, status)
    }
}

impl scrollable::Catalog for Theme {
    type Class<'a> = scrollable::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().full.min(8));

            let (hovered, dragged) = match status {
                scrollable::Status::Active { .. } => (false, false),
                scrollable::Status::Hovered { .. } => (true, false),
                scrollable::Status::Dragged { .. } => (false, true),
            };

            let scroller = if dragged {
                shade(colors.ring, 0.1)
            } else if hovered {
                colors.ring
            } else {
                colors.muted_foreground
            };

            let rail = scrollable::Rail {
                background: Some(Background::Color(colors.muted)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: radius.into(),
                },
                scroller: scrollable::Scroller {
                    background: Background::Color(fade(
                        scroller,
                        if hovered || dragged { 1.0 } else { 0.7 },
                    )),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: radius.into(),
                    },
                },
            };

            scrollable::Style {
                container: container::Style::default(),
                vertical_rail: rail,
                horizontal_rail: rail,
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: Background::Color(colors.surface),
                    border: Border {
                        color: colors.border,
                        width: 1.0,
                        radius: radius.into(),
                    },
                    shadow: Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 2.0,
                    },
                    icon: colors.foreground,
                },
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        class(self, status)
    }
}

// `menu::Catalog` extends `scrollable::Catalog`, so `Self::Class` would be
// ambiguous; every associated item below is spelled out in full.
impl pick_list::Catalog for Theme {
    type Class<'a> = pick_list::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as pick_list::Catalog>::Class<'a> {
        Box::new(|theme, status| {
            let colors = theme.colors();
            let radius = f32::from(theme.radius().md);

            let (border_color, width) = match status {
                pick_list::Status::Opened { .. } => (colors.ring, 2.0),
                pick_list::Status::Hovered => (shade(colors.input, -0.15), 1.0),
                pick_list::Status::Active => (colors.input, 1.0),
            };

            pick_list::Style {
                text_color: colors.foreground,
                placeholder_color: colors.muted_foreground,
                handle_color: colors.muted_foreground,
                background: Background::Color(colors.background),
                border: Border {
                    color: border_color,
                    width,
                    radius: radius.into(),
                },
            }
        })
    }

    fn style(
        &self,
        class: &<Self as pick_list::Catalog>::Class<'_>,
        status: pick_list::Status,
    ) -> pick_list::Style {
        class(self, status)
    }
}

// `pane_grid::Catalog` extends `container::Catalog`, so `Self::Class` would be
// ambiguous; every associated item is spelled out in full.
impl pane_grid::Catalog for Theme {
    type Class<'a> = pane_grid::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as pane_grid::Catalog>::Class<'a> {
        Box::new(|theme| {
            let colors = theme.colors();
            let tone = colors.primary;

            pane_grid::Style {
                hovered_region: pane_grid::Highlight {
                    background: Background::Color(Color { a: 0.15, ..tone }),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: f32::from(theme.radius().sm).into(),
                    },
                },
                // A picked split is emphasized over a merely hovered one, so a
                // drag in progress is unambiguous.
                picked_split: pane_grid::Line {
                    color: tone,
                    width: 2.0,
                },
                hovered_split: pane_grid::Line {
                    color: colors.ring,
                    width: 2.0,
                },
            }
        })
    }

    fn style(&self, class: &<Self as pane_grid::Catalog>::Class<'_>) -> pane_grid::Style {
        class(self)
    }
}

impl table::Catalog for Theme {
    type Class<'a> = table::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| {
            let colors = theme.colors();

            table::Style {
                separator_x: Background::Color(colors.border),
                separator_y: Background::Color(colors.border),
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> table::Style {
        class(self)
    }
}

impl markdown::Catalog for Theme {
    /// A code block sits on the muted surface, so it reads as a distinct region
    /// without needing a border.
    fn code_block<'a>() -> <Self as container::Catalog>::Class<'a> {
        Box::new(|theme: &Theme| {
            let colors = theme.colors();

            container::Style {
                background: Some(Background::Color(colors.muted)),
                border: Border {
                    color: colors.border,
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                text_color: Some(colors.foreground),
                ..container::Style::default()
            }
        })
    }
}

impl menu::Catalog for Theme {
    type Class<'a> = menu::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as menu::Catalog>::Class<'a> {
        Box::new(|theme| {
            let colors = theme.colors();

            menu::Style {
                background: Background::Color(colors.surface),
                border: Border {
                    color: colors.border,
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                text_color: colors.foreground,
                selected_text_color: colors.accent_foreground,
                selected_background: Background::Color(colors.accent),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
            }
        })
    }

    fn style(&self, class: &<Self as menu::Catalog>::Class<'_>) -> menu::Style {
        class(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{blend, fade, readable_on, shade, ButtonClass, ButtonState, ButtonVariant};
    use crate::theme::Theme;
    use iced::widget::button;
    use iced::{theme::Base, Color};

    #[test]
    fn shade_moves_toward_white_and_black() {
        let mid = Color::from_rgb8(0x80, 0x80, 0x80);
        assert!(shade(mid, 0.5).r > mid.r);
        assert!(shade(mid, -0.5).r < mid.r);
        // Shading never changes opacity.
        assert_eq!(shade(mid, 0.5).a, mid.a);
        assert_eq!(shade(mid, -0.5).a, mid.a);
    }

    #[test]
    fn fade_only_touches_alpha() {
        let color = Color::from_rgb8(0x33, 0x66, 0x99);
        let faded = fade(color, 0.5);
        assert_eq!(faded.r, color.r);
        assert_eq!(faded.a, 0.5);
    }

    #[test]
    fn blend_always_yields_an_opaque_color() {
        let top = Color::from_rgba(1.0, 0.0, 0.0, 0.5);
        let bottom = Color::from_rgb8(0x00, 0x00, 0xff);
        let mixed = blend(top, bottom);
        assert_eq!(mixed.a, 1.0);
        // Half red over blue lands between the two on both channels.
        assert!(mixed.r > 0.0 && mixed.r < 1.0);
        assert!(mixed.b > 0.0 && mixed.b < 1.0);
    }

    #[test]
    fn readable_on_flips_with_the_background() {
        assert_eq!(
            readable_on(Color::WHITE),
            Color::from_rgb8(0x0a, 0x0a, 0x0a)
        );
        assert_eq!(
            readable_on(Color::BLACK),
            Color::from_rgb8(0xfa, 0xfa, 0xfa)
        );
    }

    /// The WCAG contrast ratio between two opaque colors, from 1.0 to 21.0.
    fn contrast_ratio(a: Color, b: Color) -> f32 {
        let relative_luminance = |c: Color| {
            let channel = |v: f32| {
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
        };

        let (lighter, darker) = {
            let (la, lb) = (relative_luminance(a), relative_luminance(b));
            if la > lb {
                (la, lb)
            } else {
                (lb, la)
            }
        };

        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn foreground_colors_are_legible_on_the_backgrounds_they_sit_on() {
        // WCAG AA: 4.5:1 for body text, 3:1 for large text and UI boundaries.
        const AA_TEXT: f32 = 4.5;
        const AA_LARGE: f32 = 3.0;

        for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
            let colors = theme.colors();

            for (role, foreground, background, minimum) in [
                (
                    "foreground/background",
                    colors.foreground,
                    colors.background,
                    AA_TEXT,
                ),
                (
                    "surface_foreground/surface",
                    colors.surface_foreground,
                    colors.surface,
                    AA_TEXT,
                ),
                (
                    "primary_foreground/primary",
                    colors.primary_foreground,
                    colors.primary,
                    AA_TEXT,
                ),
                (
                    "secondary_foreground/secondary",
                    colors.secondary_foreground,
                    colors.secondary,
                    AA_TEXT,
                ),
                (
                    "accent_foreground/accent",
                    colors.accent_foreground,
                    colors.accent,
                    AA_TEXT,
                ),
                // `muted_foreground` is a deliberately de-emphasized role:
                // upstream shadcn/ui and gpui-kit both pair it at roughly
                // 4.35:1 in light mode, so it is held to the large-text bar
                // rather than silently diverging from the source palette.
                (
                    "muted_foreground/muted",
                    colors.muted_foreground,
                    colors.muted,
                    AA_LARGE,
                ),
            ] {
                let ratio = contrast_ratio(foreground, background);
                assert!(
                    ratio >= minimum,
                    "{name} theme: {role} has a contrast ratio of {ratio:.2}, below {minimum}"
                );
            }
        }
    }

    #[test]
    fn destructive_foreground_is_legible_on_destructive() {
        // The gpui-kit dark palette pairs `destructive` with itself, so this
        // only holds where the two actually differ.
        let theme = Theme::light();
        let colors = theme.colors();
        let ratio = contrast_ratio(colors.destructive_foreground, colors.destructive);
        assert!(
            ratio >= 3.0,
            "light theme: destructive pair has a contrast ratio of {ratio:.2}"
        );
    }

    #[test]
    fn every_button_variant_produces_a_style_in_every_status() {
        for theme in [Theme::light(), Theme::dark()] {
            for variant in [
                ButtonVariant::Default,
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Danger,
                ButtonVariant::Warning,
                ButtonVariant::Success,
                ButtonVariant::Info,
                ButtonVariant::Destructive,
                ButtonVariant::Ghost,
                ButtonVariant::Link,
                ButtonVariant::Text,
            ] {
                for status in [
                    button::Status::Active,
                    button::Status::Hovered,
                    button::Status::Pressed,
                    button::Status::Disabled,
                ] {
                    let style = super::button_style(&theme, variant, crate::theme::Size::Md);
                    let _ = style(&status);
                }
            }
        }
    }

    /// A selection has to be visible on a filled variant. Darkening an
    /// already near-black `Primary` fill is imperceptible, so the palette alone
    /// cannot carry the signal — the ring is what does.
    #[test]
    fn a_selected_button_is_visibly_different_from_a_resting_one() {
        for theme in [Theme::light(), Theme::dark()] {
            for variant in [
                ButtonVariant::Default,
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Danger,
                ButtonVariant::Success,
            ] {
                let resting = ButtonClass {
                    variant,
                    ..ButtonClass::default()
                }
                .appearance(&theme, ButtonState::Normal);

                let selected = ButtonClass {
                    variant,
                    selected: true,
                    ..ButtonClass::default()
                }
                .appearance(&theme, ButtonState::Selected);

                assert!(
                    selected.border.width > resting.border.width,
                    "{variant:?} in {:?}: a selected button must ring itself",
                    theme.mode()
                );
            }
        }
    }

    /// The soft tones must be distinguishable from the filled destructive
    /// variant: they are different signals, not different names for the same
    /// thing.
    #[test]
    fn the_soft_danger_tone_differs_from_the_filled_destructive_one() {
        let theme = Theme::light();

        let soft = ButtonClass {
            variant: ButtonVariant::Danger,
            ..ButtonClass::default()
        }
        .appearance(&theme, ButtonState::Normal);

        let filled = ButtonClass {
            variant: ButtonVariant::Destructive,
            ..ButtonClass::default()
        }
        .appearance(&theme, ButtonState::Normal);

        assert_ne!(soft.background, filled.background);
        assert_ne!(soft.text_color, filled.text_color);
    }

    /// A link is underlined in every state; it is the property that makes it
    /// read as a link rather than as plain text.
    #[test]
    fn a_link_variant_asks_to_be_underlined() {
        assert!(ButtonVariant::Link.underline());

        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Text,
            ButtonVariant::Ghost,
        ] {
            assert!(!variant.underline(), "{variant:?} is not a link");
        }
    }

    #[test]
    fn disabled_buttons_are_translucent_and_enabled_ones_are_not() {
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Destructive,
        ] {
            let theme = Theme::light();
            let style = super::button_style(&theme, variant, crate::theme::Size::Md);

            let enabled = style(&button::Status::Active);
            let disabled = style(&button::Status::Disabled);

            let alpha = |s: &button::Style| match s.background {
                Some(iced::Background::Color(c)) => c.a,
                _ => 1.0,
            };

            assert!(
                alpha(&disabled) < alpha(&enabled),
                "{variant:?} should look dimmer when disabled"
            );
        }
    }

    #[test]
    fn theme_supplies_a_base_style_for_stock_widgets() {
        let base = Theme::dark().base();
        assert_eq!(base.background_color, Theme::dark().colors().background);
    }
}

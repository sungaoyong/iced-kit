//! Presentational components: cards, dividers, badges, progress and alerts.

use crate::theme::{catalog, Size, Theme};
use iced::widget::{container, progress_bar as iced_progress_bar, rule, text};
use iced::{Color, Element, Length, Padding};

/// The emphasis of a [`badge`] or [`alert`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// A neutral, informational tone.
    #[default]
    Neutral,
    /// A positive tone.
    Success,
    /// A cautionary tone.
    Warning,
    /// A destructive or erroneous tone.
    Danger,
    /// The brand tone.
    Primary,
}

impl Tone {
    /// The accent color this tone uses for text and icons.
    pub(crate) fn accent(self, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Neutral => colors.muted_foreground,
            Self::Success => Color::from_rgb8(0x16, 0xa3, 0x4a),
            Self::Warning => Color::from_rgb8(0xd9, 0x77, 0x06),
            Self::Danger => colors.destructive,
            Self::Primary => colors.primary,
        }
    }

    /// The tinted background this tone uses on a filled surface.
    ///
    /// The tint is derived from [`Tone::accent`] so that a custom palette's
    /// colors flow through instead of hard-coded pastels.
    pub(crate) fn surface(self, theme: &Theme) -> Color {
        let colors = theme.colors();
        let accent = self.accent(theme);
        let base = if theme.is_dark() {
            colors.background
        } else {
            colors.surface
        };

        // Blend the accent into the surface at low alpha. `Neutral` is already
        // a surface color, so it needs its own treatment.
        let tint_alpha = if self == Self::Neutral { 0.0 } else { 0.12 };

        Color {
            r: accent.r * tint_alpha + base.r * (1.0 - tint_alpha),
            g: accent.g * tint_alpha + base.g * (1.0 - tint_alpha),
            b: accent.b * tint_alpha + base.b * (1.0 - tint_alpha),
            a: 1.0,
        }
    }

    /// The border color this tone uses.
    ///
    /// A neutral tone draws no border, so it returns the accent at zero alpha
    /// and callers blend it themselves.
    pub(crate) fn border(self, theme: &Theme) -> Color {
        let alpha = if self == Self::Neutral { 0.0 } else { 0.4 };

        Color {
            a: alpha,
            ..self.accent(theme)
        }
    }
}

/// Builds a card: a raised surface with a border and rounded corners.
///
/// ```
/// # use iced_kit::widgets::{card, label};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// card(label("Account")).into()
/// # }
/// ```
pub fn card<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
) -> container::Container<'a, Message, Theme> {
    container(content)
        .padding(16)
        .class(Box::new(catalog::card) as container::StyleFn<'a, Theme>)
}

/// Builds a horizontal divider.
///
/// A divider carries no message type, so it can be dropped into any view.
pub fn divider<'a>() -> rule::Rule<'a, Theme> {
    rule::horizontal(1)
}

/// Builds a vertical divider.
pub fn vertical_divider<'a>() -> rule::Rule<'a, Theme> {
    rule::vertical(1)
}

/// Builds a small, pill-shaped status label.
///
/// ```
/// # use iced_kit::widgets::{badge, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// badge("Active", Tone::Success).into()
/// # }
/// ```
pub fn badge<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    tone: Tone,
) -> container::Container<'a, Message, Theme> {
    let text_size = Size::Sm.text();

    container(
        text(label)
            .size(text_size.size)
            .line_height(text_size.line_height()),
    )
    .padding(Padding {
        top: 2.0,
        right: 8.0,
        bottom: 2.0,
        left: 8.0,
    })
    .class(Box::new(move |theme: &Theme| {
        let accent = tone.accent(theme);

        container::Style {
            background: Some(iced::Background::Color(tone.surface(theme))),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().full.min(999)).into(),
            },
            text_color: Some(accent),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
}

/// Builds a labelled progress bar.
///
/// The value is clamped to the `0.0..=1.0` range; passing a percentage is a
/// common mistake that would otherwise silently overflow the track.
pub fn progress<'a, Message: 'a>(value: f32, tone: Tone) -> Element<'a, Message, Theme> {
    iced_progress_bar(0.0..=1.0, value.clamp(0.0, 1.0))
        .length(Length::Fill)
        .girth(8)
        .class(Box::new(move |theme: &Theme| {
            let colors = theme.colors();

            iced_progress_bar::Style {
                background: iced::Background::Color(colors.secondary),
                bar: iced::Background::Color(match tone {
                    Tone::Neutral => colors.primary,
                    other => other.accent(theme),
                }),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: f32::from(theme.radius().full.min(8)).into(),
                },
            }
        }) as iced_progress_bar::StyleFn<'a, Theme>)
        .into()
}

/// Builds an alert: a full-width message with a tinted background.
///
/// `title` and `body` are separate so the caller can give the alert a heading;
/// pass the same string to both for a single-line alert.
///
/// ```
/// # use iced_kit::widgets::{alert, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # use iced::widget::column;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// alert("Heads up", "Your trial ends in 3 days.", Tone::Warning).into()
/// # }
/// ```
pub fn alert<'a, Message: 'a>(
    title: impl text::IntoFragment<'a>,
    body: impl text::IntoFragment<'a>,
    tone: Tone,
) -> Element<'a, Message, Theme> {
    let title_size = Size::Md.text();
    let body_size = Size::Sm.text();

    container(
        iced::widget::column![
            text(title)
                .size(title_size.size)
                .line_height(title_size.line_height()),
            text(body)
                .size(body_size.size)
                .line_height(body_size.line_height()),
        ]
        .spacing(4),
    )
    .padding(16)
    .width(Length::Fill)
    .class(Box::new(move |theme: &Theme| {
        let colors = theme.colors();
        let surface = tone.surface(theme);

        container::Style {
            background: Some(iced::Background::Color(surface)),
            border: iced::Border {
                // The accent is translucent, so it is composited onto the
                // tinted surface to stay visible against it.
                color: catalog::blend(tone.border(theme), surface),
                width: 1.0,
                radius: f32::from(theme.radius().md).into(),
            },
            // The tinted surface is the signal; the text stays the theme's
            // foreground so it keeps its contrast on a custom palette.
            text_color: Some(colors.foreground),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
    .into()
}

/// Builds an empty-state placeholder: a centered, muted message.
pub fn empty_state<'a, Message: 'a>(
    title: impl text::IntoFragment<'a>,
    description: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let title_size = Size::Md.text();
    let body_size = Size::Sm.text();

    container(
        iced::widget::column![
            text(title)
                .size(title_size.size)
                .line_height(title_size.line_height()),
            text(description)
                .size(body_size.size)
                .line_height(body_size.line_height()),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center),
    )
    .padding(32)
    .width(Length::Fill)
    .center_x(Length::Fill)
    .class(Box::new(catalog::muted) as container::StyleFn<'a, Theme>)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{alert, badge, card, divider, empty_state, progress, Tone};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    #[test]
    fn a_card_renders() {
        let element: iced::Element<'_, Message, Theme> = card("Content").into();
        drop(element);
    }

    #[test]
    fn dividers_render_in_both_orientations() {
        let horizontal: iced::Element<'_, Message, Theme> = divider().into();
        let vertical: iced::Element<'_, Message, Theme> = super::vertical_divider().into();
        drop(horizontal);
        drop(vertical);
    }

    #[test]
    fn a_badge_renders_in_every_tone() {
        for tone in [
            Tone::Neutral,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
            Tone::Primary,
        ] {
            let element: iced::Element<'_, Message, Theme> = badge("Active", tone).into();
            drop(element);
        }
    }

    #[test]
    fn progress_clamps_out_of_range_values() {
        // Values outside 0..=1 are clamped rather than panicking, because a
        // caller passing a percentage should not crash the UI.
        for value in [-1.0, 0.0, 0.5, 1.0, 2.0, f32::NAN] {
            let element: iced::Element<'_, Message, Theme> = progress(value, Tone::Primary);
            drop(element);
        }
    }

    #[test]
    fn an_alert_renders_in_every_tone() {
        for tone in [Tone::Neutral, Tone::Success, Tone::Warning, Tone::Danger] {
            let element: iced::Element<'_, Message, Theme> = alert("Title", "Body", tone);
            drop(element);
        }
    }

    #[test]
    fn an_empty_state_renders() {
        let element: iced::Element<'_, Message, Theme> = empty_state("Nothing here", "Add an item");
        drop(element);
    }

    #[test]
    fn tone_accents_are_distinct_where_they_should_be() {
        let theme = Theme::light();
        assert_ne!(Tone::Success.accent(&theme), Tone::Danger.accent(&theme));
        assert_ne!(Tone::Warning.accent(&theme), Tone::Success.accent(&theme));
    }

    #[test]
    fn a_neutral_tone_adds_no_tint() {
        let theme = Theme::light();
        // A neutral surface must not be tinted, or a plain card would look
        // like an alert.
        let surface = Tone::Neutral.surface(&theme);
        let base = theme.colors().surface;
        assert_eq!(surface, base);
    }
}

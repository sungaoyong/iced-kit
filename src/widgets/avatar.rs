//! Avatars: a user's initials on a tinted circle.

use crate::theme::{Size, Theme};
use iced::widget::{container, text};
use iced::{Color, Element, Length};

/// The shape of an [`avatar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarShape {
    /// A circle, the familiar user-avatar shape.
    #[default]
    Circle,
    /// A rounded square.
    Square,
}

/// Builds an avatar showing a person's (or team's) initials.
///
/// Up to two initials are rendered; extra characters are dropped so a long
/// display name cannot overflow the circle.
///
/// ```
/// # use iced_kit::widgets::avatar;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// avatar("Ada Lovelace", 40)
/// # }
/// ```
pub fn avatar<'a, Message: 'a>(
    name: impl text::IntoFragment<'a>,
    diameter: u16,
) -> Element<'a, Message, Theme> {
    let label = initials(name.into_fragment().as_ref());
    avatar_with_label(label, diameter)
}

/// Builds an avatar from an explicit label rather than a display name.
///
/// Use this when the initials are already known, so no name has to be parsed.
pub fn avatar_with_label<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    diameter: u16,
) -> Element<'a, Message, Theme> {
    let font_size = (f32::from(diameter) * 0.4).max(9.0);
    let radius = match AvatarShape::default() {
        AvatarShape::Circle => f32::from(diameter) / 2.0,
        AvatarShape::Square => f32::from(diameter) * 0.28,
    };

    container(
        text(label)
            .size(font_size)
            .line_height(iced::Pixels(font_size * 1.2))
            .font(iced::Font {
                weight: iced::font::Weight::Medium,
                ..iced::Font::DEFAULT
            }),
    )
    .width(Length::Fixed(f32::from(diameter)))
    .height(Length::Fixed(f32::from(diameter)))
    .center_x(Length::Fixed(f32::from(diameter)))
    .center_y(Length::Fixed(f32::from(diameter)))
    .class(Box::new(move |theme: &Theme| {
        let colors = theme.colors();
        // The primary token carries the identity tint, so a custom palette
        // recolors avatars along with the rest of the UI.
        let tint = colors.primary;

        container::Style {
            background: Some(iced::Background::Color(Color { a: 0.15, ..tint })),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius.into(),
            },
            text_color: Some(readable_on_tint(tint, colors.background)),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
    .into()
}

/// A label placed next to an avatar.
#[derive(Debug, Clone)]
#[must_use = "an AvatarLabel does nothing unless it is given to `avatar_with_name`"]
pub struct AvatarLabel {
    name: String,
    detail: Option<String>,
}

impl AvatarLabel {
    /// Creates a label with a name and an optional secondary line.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            detail: None,
        }
    }

    /// Sets the secondary line, such as an email address.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Builds an avatar with its name (and optional detail) beside it.
pub fn avatar_with_name<'a, Message: 'a>(
    label: AvatarLabel,
    diameter: u16,
    size: Size,
) -> Element<'a, Message, Theme> {
    let name_style = size.text();
    let detail_style = Size::Sm.text();

    let avatar_element: Element<'a, Message, Theme> =
        avatar_with_label(initials(&label.name), diameter);

    let mut column = iced::widget::column![text(label.name)
        .size(name_style.size)
        .line_height(name_style.line_height())]
    .spacing(2);

    if let Some(detail) = label.detail {
        column = column.push(
            text(detail)
                .size(detail_style.size)
                .line_height(detail_style.line_height())
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as text::StyleFn<'a, Theme>),
        );
    }

    iced::widget::row![avatar_element, column]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Picks a legible color for text drawn on a lightly tinted background.
///
/// The avatar background is the accent at 15% over the page background, which
/// is far lighter than the accent itself, so the accent cannot be used as-is.
fn readable_on_tint(accent: Color, background: Color) -> Color {
    const TINT: f32 = 0.15;

    let tinted = Color {
        r: accent.r * TINT + background.r * (1.0 - TINT),
        g: accent.g * TINT + background.g * (1.0 - TINT),
        b: accent.b * TINT + background.b * (1.0 - TINT),
        a: 1.0,
    };

    let luminance = 0.2126 * tinted.r + 0.7152 * tinted.g + 0.0722 * tinted.b;

    if luminance > 0.5 {
        // Darken the accent so it reads on a pale tint.
        Color {
            r: accent.r * 0.45,
            g: accent.g * 0.45,
            b: accent.b * 0.45,
            a: 1.0,
        }
    } else {
        // Lighten it for a dark tint.
        Color {
            r: accent.r + (1.0 - accent.r) * 0.55,
            g: accent.g + (1.0 - accent.g) * 0.55,
            b: accent.b + (1.0 - accent.b) * 0.55,
            a: 1.0,
        }
    }
}

/// Extracts up to two initials from a display name.
///
/// Splits on whitespace and takes the first character of the first and last
/// words, so "Ada Lovelace" yields "AL" and "Ada" yields "A". A name with no
/// alphabetic characters falls back to "?" so the avatar is never blank.
fn initials(name: &str) -> String {
    let mut words = name
        .split_whitespace()
        .filter_map(|word| word.chars().next());

    let first = match words.next() {
        Some(c) if c.is_alphanumeric() => c,
        _ => return "?".to_owned(),
    };

    // The last word is what distinguishes "Ada Lovelace" from "Ada B. Lovelace".
    let last = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .rfind(|c: &char| c.is_alphanumeric());

    match last {
        Some(last) if last != first || name.split_whitespace().count() > 1 => {
            let mut result = String::with_capacity(2);
            result.push(first.to_ascii_uppercase());
            result.push(last.to_ascii_uppercase());
            result
        }
        _ => first.to_ascii_uppercase().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{avatar, avatar_with_label, avatar_with_name, initials, AvatarLabel};

    #[test]
    fn initials_use_the_first_and_last_words() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("Grace Brewster Murray Hopper"), "GH");
        assert_eq!(initials("Ada"), "A");
    }

    #[test]
    fn initials_are_uppercased() {
        assert_eq!(initials("ada lovelace"), "AL");
    }

    #[test]
    fn initials_ignore_extra_whitespace() {
        assert_eq!(initials("  Ada   Lovelace  "), "AL");
    }

    #[test]
    fn a_nameless_avatar_falls_back_rather_than_going_blank() {
        assert_eq!(initials(""), "?");
        assert_eq!(initials("   "), "?");
        assert_eq!(initials("123"), "1");
    }

    #[test]
    fn an_avatar_renders_at_several_sizes() {
        for diameter in [16, 24, 40, 96] {
            let element: iced::Element<'_, (), crate::theme::Theme> =
                avatar("Ada Lovelace", diameter);
            drop(element);
        }
    }

    #[test]
    fn a_labelled_avatar_renders_with_and_without_a_detail_line() {
        let simple: iced::Element<'_, (), crate::theme::Theme> =
            avatar_with_name(AvatarLabel::new("Ada Lovelace"), 40, crate::theme::Size::Md);
        drop(simple);

        let detailed: iced::Element<'_, (), crate::theme::Theme> = avatar_with_name(
            AvatarLabel::new("Ada Lovelace").detail("ada@example.com"),
            40,
            crate::theme::Size::Lg,
        );
        drop(detailed);
    }

    #[test]
    fn a_precomputed_label_avatar_renders() {
        let element: iced::Element<'_, (), crate::theme::Theme> = avatar_with_label("AL", 32);
        drop(element);
    }

    #[test]
    fn tinted_text_is_legible_in_both_palettes() {
        use crate::theme::Theme;
        use iced::theme::Base;

        let contrast = |a: iced::Color, b: iced::Color| {
            let lum = |c: iced::Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
            let (hi, lo) = {
                let (x, y) = (lum(a), lum(b));
                if x > y {
                    (x, y)
                } else {
                    (y, x)
                }
            };
            (hi + 0.05) / (lo + 0.05)
        };

        for theme in [Theme::light(), Theme::dark()] {
            let base = theme.base();
            let tinted = super::readable_on_tint(theme.colors().primary, base.background_color);
            let ratio = contrast(tinted, base.background_color);

            assert!(
                ratio >= 3.0,
                "avatar initials need enough contrast to read, got {ratio:.2}"
            );
        }
    }
}

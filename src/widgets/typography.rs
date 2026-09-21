//! Text primitives: typography and keyboard keys.

use crate::theme::{Size, Theme};
use iced::widget::{container, text};
use iced::{Color, Element, Length, Padding};

/// A heading level, following the HTML convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Heading {
    /// The largest heading.
    H1,
    /// A section heading.
    H2,
    /// A subsection heading.
    #[default]
    H3,
    /// The smallest heading.
    H4,
}

impl Heading {
    /// The type step this heading uses.
    #[must_use]
    pub const fn text_style(self) -> iced::widget::text::LineHeight {
        match self {
            Self::H1 => iced::widget::text::LineHeight::Absolute(iced::Pixels(36.0)),
            Self::H2 => iced::widget::text::LineHeight::Absolute(iced::Pixels(30.0)),
            Self::H3 => iced::widget::text::LineHeight::Absolute(iced::Pixels(24.0)),
            Self::H4 => iced::widget::text::LineHeight::Absolute(iced::Pixels(20.0)),
        }
    }

    /// The font size this heading uses.
    #[must_use]
    pub const fn size(self) -> f32 {
        match self {
            Self::H1 => 28.0,
            Self::H2 => 23.0,
            Self::H3 => 19.0,
            Self::H4 => 16.0,
        }
    }
}

/// Builds a heading.
///
/// ```
/// # use iced_kit::widgets::{heading, Heading};
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// heading("Dashboard", Heading::H2).into()
/// # }
/// ```
pub fn heading<'a>(content: impl text::IntoFragment<'a>, level: Heading) -> text::Text<'a, Theme> {
    text(content)
        .size(level.size())
        .line_height(level.text_style())
}

/// Builds body text at the default size.
pub fn paragraph<'a>(content: impl text::IntoFragment<'a>) -> text::Text<'a, Theme> {
    let style = Size::Md.text();

    text(content)
        .size(style.size)
        .line_height(style.line_height())
}

/// Builds de-emphasized helper text, such as a field hint.
pub fn muted_text<'a>(content: impl text::IntoFragment<'a>) -> text::Text<'a, Theme> {
    let style = Size::Sm.text();

    text(content)
        .size(style.size)
        .line_height(style.line_height())
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as text::StyleFn<'a, Theme>)
}

/// Builds inline code text, rendered in the theme's monospace font.
pub fn code<'a, Message: 'a>(
    content: impl text::IntoFragment<'a>,
) -> container::Container<'a, Message, Theme> {
    let style = Size::Sm.text();

    container(
        text(content)
            .size(style.size)
            .line_height(style.line_height())
            .font(iced::Font::MONOSPACE),
    )
    .padding(Padding {
        top: 2.0,
        right: 6.0,
        bottom: 2.0,
        left: 6.0,
    })
    .class(Box::new(|theme: &Theme| {
        let colors = theme.colors();

        container::Style {
            background: Some(iced::Background::Color(colors.muted)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().sm).into(),
            },
            text_color: Some(colors.foreground),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
}

/// Builds a keyboard key hint, such as `Ctrl` or `K`.
///
/// ```
/// # use iced_kit::widgets::kbd;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// kbd("Ctrl")
/// # }
/// ```
pub fn kbd<'a, Message: 'a>(key: impl text::IntoFragment<'a>) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    container(
        text(key)
            .size(style.size - 1.0)
            .line_height(style.line_height())
            .font(iced::Font::MONOSPACE),
    )
    .padding(Padding {
        top: 2.0,
        right: 6.0,
        bottom: 2.0,
        left: 6.0,
    })
    .class(Box::new(|theme: &Theme| {
        let colors = theme.colors();

        container::Style {
            background: Some(iced::Background::Color(colors.muted)),
            border: iced::Border {
                color: colors.border,
                width: 1.0,
                radius: f32::from(theme.radius().sm).into(),
            },
            text_color: Some(colors.muted_foreground),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
    .into()
}

/// Builds a keyboard shortcut hint: several keys with `+` between them.
pub fn shortcut<'a, Message: 'a>(keys: &[&'a str]) -> Element<'a, Message, Theme> {
    let mut row = iced::widget::row![]
        .spacing(4)
        .align_y(iced::Alignment::Center);

    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            row = row.push(muted_text("+"));
        }
        row = row.push(kbd::<Message>(*key));
    }

    row.into()
}

/// Builds a block of text separated from its surroundings by a rule.
pub fn section_label<'a, Message: 'a>(
    content: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    text(content)
        .size(style.size)
        .line_height(style.line_height())
        .width(Length::Fill)
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as text::StyleFn<'a, Theme>)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{code, heading, kbd, muted_text, paragraph, section_label, shortcut, Heading};
    use crate::theme::Theme;

    /// The message type every render below is built against.
    type Message = ();

    #[test]
    fn headings_render_at_every_level() {
        for level in [Heading::H1, Heading::H2, Heading::H3, Heading::H4] {
            let element: iced::Element<'_, Message, Theme> = heading("Title", level).into();
            drop(element);
        }
    }

    #[test]
    fn heading_sizes_shrink_as_the_level_deepens() {
        assert!(Heading::H1.size() > Heading::H2.size());
        assert!(Heading::H2.size() > Heading::H3.size());
        assert!(Heading::H3.size() > Heading::H4.size());
    }

    #[test]
    fn body_text_renders() {
        let element: iced::Element<'_, Message, Theme> = paragraph("Body text").into();
        drop(element);

        let element: iced::Element<'_, Message, Theme> = muted_text("Hint").into();
        drop(element);

        let element: iced::Element<'_, Message, Theme> = code("let x = 1;").into();
        drop(element);

        let element: iced::Element<'_, Message, Theme> = section_label("GENERAL");
        drop(element);
    }

    #[test]
    fn a_key_hint_renders() {
        let element: iced::Element<'_, Message, Theme> = kbd::<Message>("Ctrl");
        drop(element);
    }

    #[test]
    fn a_shortcut_renders_with_and_without_separators() {
        let single: iced::Element<'_, Message, Theme> = shortcut::<Message>(&["Esc"]);
        drop(single);

        let combo: iced::Element<'_, Message, Theme> = shortcut::<Message>(&["Ctrl", "Shift", "K"]);
        drop(combo);

        let none: iced::Element<'_, Message, Theme> = shortcut::<Message>(&[]);
        drop(none);
    }
}

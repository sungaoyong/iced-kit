//! Text primitives: typography and keyboard keys.

use crate::theme::{Size, Theme};
use iced::widget::{container, text};
use iced::{Alignment, Color, Element, Length, Padding};

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

/// What a [`Label`] highlights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightsMatch {
    /// Highlight only when the text starts with the query.
    Prefix(String),
    /// Highlight every occurrence of the query.
    Full(String),
}

impl HighlightsMatch {
    /// The text being looked for.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prefix(text) | Self::Full(text) => text,
        }
    }

    /// Whether the match is anchored at the start.
    #[must_use]
    pub fn is_prefix(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
}

impl From<&str> for HighlightsMatch {
    fn from(value: &str) -> Self {
        Self::Full(value.to_owned())
    }
}

impl From<String> for HighlightsMatch {
    fn from(value: String) -> Self {
        Self::Full(value)
    }
}

/// The character a masked label hides its text behind.
const MASK: char = '\u{2022}';

/// A text label with an optional secondary line, masking and search
/// highlighting.
///
/// [`paragraph`] and [`muted_text`] cover plain text; this builder adds what
/// the reference's `Label` carries. Masking is what a password field's value
/// uses, and highlighting is what marks a search hit.
///
/// ```
/// # use iced_kit::widgets::{label_builder, HighlightsMatch};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// // A masked value, and a name with the search term marked.
/// label_builder("hunter2").masked(true).into()
/// # }
/// ```
#[must_use = "a Label does nothing unless it is turned into an Element"]
pub struct Label<'a, Message> {
    text: String,
    secondary: Option<String>,
    masked: bool,
    highlights: Option<HighlightsMatch>,
    color: Option<Color>,
    size: Size,
    _message: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> Label<'a, Message> {
    /// Creates a label with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            secondary: None,
            masked: false,
            highlights: None,
            color: None,
            size: Size::Md,
            _message: std::marker::PhantomData,
        }
    }

    /// Adds a secondary, muted line after the primary text.
    pub fn secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    /// Hides the text behind bullets.
    ///
    /// The secondary line is not masked: it carries a label such as a user
    /// name, which is there to say whose secret this is.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Marks a search term's occurrences.
    ///
    /// iced's text widget carries one style for a whole run, so the match is
    /// not drawn as a highlighted span within the line. A hit is drawn on the
    /// accent fill instead, which is what makes it stand out in a list — the
    /// same treatment a selected row gets. A caller that needs the matched span
    /// picked out draws the pieces as separate texts.
    pub fn highlights(mut self, matched: impl Into<HighlightsMatch>) -> Self {
        self.highlights = Some(matched.into());
        self
    }

    /// Overrides the label's colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the size step.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// The text the label draws, masked or not.
    #[must_use]
    pub fn display_text(&self) -> String {
        if self.masked {
            MASK.to_string().repeat(self.text.chars().count())
        } else {
            self.text.clone()
        }
    }

    /// Whether the label's text occurs in `query`, by its match rule.
    #[must_use]
    pub fn matches(&self, query: &HighlightsMatch) -> bool {
        let haystack = self.text.to_lowercase();
        let needle = query.as_str().to_lowercase();

        if needle.is_empty() {
            return false;
        }

        if query.is_prefix() {
            haystack.starts_with(&needle)
        } else {
            haystack.contains(&needle)
        }
    }

    /// Whether this label is a search hit, if it was given a term.
    #[must_use]
    pub fn is_highlighted(&self) -> bool {
        self.highlights
            .as_ref()
            .is_some_and(|query| self.matches(query))
    }

    /// Turns the label into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let style = self.size.text();
        let color = self.color;
        let highlighted = self.is_highlighted();

        let primary: Element<'a, Message, Theme> = text(self.display_text())
            .size(style.size)
            .line_height(style.line_height())
            .class(Box::new(move |theme: &Theme| text::Style {
                color: Some(color.unwrap_or(theme.colors().foreground)),
            }) as text::StyleFn<'a, Theme>)
            .into();

        // A hit is drawn on the accent fill, the same treatment a selected row
        // gets. Colouring the text alone cannot work: every palette's link and
        // foreground tokens are the same shade, so a highlighted label would
        // look exactly like an ordinary one.
        let primary: Element<'a, Message, Theme> = if highlighted {
            container(primary)
                .padding(Padding {
                    top: 0.0,
                    right: 4.0,
                    bottom: 0.0,
                    left: 4.0,
                })
                .class(Box::new(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.colors().accent)),
                    border: iced::Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: f32::from(theme.radius().sm).into(),
                    },
                    text_color: Some(theme.colors().accent_foreground),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>)
                .into()
        } else {
            primary
        };

        let Some(secondary) = self.secondary else {
            return primary;
        };

        // A masked value is followed by its label, which says whose it is.
        let secondary_text = text(secondary)
            .size(Size::Sm.text().size)
            .line_height(Size::Sm.text().line_height())
            .class(Box::new(|theme: &Theme| text::Style {
                color: Some(theme.colors().muted_foreground),
            }) as text::StyleFn<'a, Theme>);

        iced::widget::row![primary, secondary_text]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }
}

impl<'a, Message: 'a> From<Label<'a, Message>> for Element<'a, Message, Theme> {
    fn from(label: Label<'a, Message>) -> Self {
        label.into_element()
    }
}

/// Builds a label.
pub fn label_builder<'a, Message: 'a>(text: impl Into<String>) -> Label<'a, Message> {
    Label::new(text)
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
    use super::{
        code, heading, kbd, label_builder, muted_text, paragraph, section_label, shortcut, Heading,
        HighlightsMatch, Label,
    };
    use crate::theme::{Size, Theme};

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

    #[test]
    fn a_label_masks_its_text() {
        let visible: Label<'_, Message> = label_builder("hunter2");
        assert_eq!(visible.display_text(), "hunter2");

        let masked: Label<'_, Message> = label_builder("hunter2").masked(true);
        assert_eq!(masked.display_text(), "\u{2022}".repeat(7));

        // A masked label keeps its secondary line, which names whose value it
        // is rather than repeating it.
        let with_name: Label<'_, Message> = label_builder("hunter2").masked(true).secondary("Ada");
        assert_eq!(with_name.secondary.as_deref(), Some("Ada"));
    }

    #[test]
    fn a_label_reports_its_search_hits() {
        let label: Label<'_, Message> = label_builder("Ada Lovelace");

        assert!(label.matches(&HighlightsMatch::Full("love".to_owned())));
        assert!(!label.matches(&HighlightsMatch::Full("grace".to_owned())));
        assert!(!label.matches(&HighlightsMatch::Full(String::new())));

        // A prefix match is anchored; a full match is not.
        assert!(label.matches(&HighlightsMatch::Prefix("ada".to_owned())));
        assert!(!label.matches(&HighlightsMatch::Prefix("love".to_owned())));
    }

    #[test]
    fn a_label_is_highlighted_only_when_it_was_given_a_term() {
        let plain: Label<'_, Message> = label_builder("Ada Lovelace");
        assert!(!plain.is_highlighted());

        let hit: Label<'_, Message> =
            label_builder("Ada Lovelace").highlights(HighlightsMatch::Full("ada".to_owned()));
        assert!(hit.is_highlighted());

        let miss: Label<'_, Message> =
            label_builder("Ada Lovelace").highlights(HighlightsMatch::Full("grace".to_owned()));
        assert!(!miss.is_highlighted());
    }

    #[test]
    fn a_highlight_match_takes_a_bare_string_as_a_full_match() {
        let from_str: HighlightsMatch = "ada".into();
        assert_eq!(from_str, HighlightsMatch::Full("ada".to_owned()));
        assert!(!from_str.is_prefix());

        let from_string: HighlightsMatch = String::from("ada").into();
        assert_eq!(from_string.as_str(), "ada");
    }

    #[test]
    fn labels_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            label_builder("Plain").into(),
            label_builder("Titled").secondary("detail").into(),
            label_builder("hunter2").masked(true).into(),
            label_builder("Ada")
                .highlights(HighlightsMatch::Full("ada".to_owned()))
                .into(),
            label_builder("Ada")
                .color(iced::Color::WHITE)
                .size(Size::Lg)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }
}

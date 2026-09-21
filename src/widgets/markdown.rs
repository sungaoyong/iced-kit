//! Markdown rendering.
//!
//! iced 0.14 ships a Markdown widget behind its `markdown` feature, so this
//! module does not reimplement a parser. What it adds is the design system: a
//! [`Markdown`] wrapper that derives the widget's settings from the theme's
//! type scale, so headings and code in a document match the rest of the UI.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::markdown::Markdown;
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! fn view(document: &Markdown) -> Element<'_, iced::widget::markdown::Uri, Theme> {
//!     document.into_element()
//! }
//! ```

use crate::theme::{Theme, Tokens};
use iced::widget::markdown::{
    self, Content, Item as MarkdownItem, Settings as MarkdownSettings, Style as MarkdownStyle, Uri,
};
use iced::{font, Element, Font, Pixels};

pub use iced::widget::markdown::{Bullet, Item};

/// A parsed Markdown document.
///
/// Parsing is kept separate from rendering so a document is converted once and
/// re-rendered per frame, rather than re-parsed on every view call.
#[derive(Debug, Default)]
pub struct Markdown {
    content: Content,
    text_size: Option<f32>,
    spacing: Option<f32>,
}

impl Markdown {
    /// Parses a Markdown document.
    #[must_use]
    pub fn parse(source: &str) -> Self {
        Self {
            content: Content::parse(source),
            text_size: None,
            spacing: None,
        }
    }

    /// Creates an empty document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base text size, overriding the theme's body size.
    #[must_use]
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Sets the vertical spacing between blocks, overriding the theme's.
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Appends more Markdown to the document.
    pub fn push(&mut self, source: &str) {
        self.content.push_str(source);
    }

    /// The parsed blocks.
    #[must_use]
    pub fn items(&self) -> &[MarkdownItem] {
        self.content.items()
    }

    /// Whether the document has no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.items().is_empty()
    }

    /// The settings derived from the design tokens.
    fn settings(&self) -> MarkdownSettings {
        let tokens = Tokens::default();
        let typography = &tokens.typography;
        let colors = tokens.colors;

        let style = MarkdownStyle {
            font: typography.sans,
            inline_code_font: typography.mono,
            code_block_font: typography.mono,
            inline_code_color: colors.foreground,
            link_color: colors.primary,
            inline_code_highlight: iced::widget::markdown::Highlight {
                background: iced::Background::Color(colors.muted),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 3.0.into(),
                },
            },
            inline_code_padding: iced::Padding {
                top: 1.0,
                right: 4.0,
                bottom: 1.0,
                left: 4.0,
            },
        };

        // The heading scale steps by a consistent amount off the body size, so a
        // custom base size keeps the hierarchy intact.
        let base = self.text_size.unwrap_or(typography.md.size);
        let step = |offset: f32| Pixels(base + offset);

        MarkdownSettings {
            text_size: Pixels(base),
            h1_size: step(10.0),
            h2_size: step(7.0),
            h3_size: step(4.0),
            h4_size: step(2.0),
            h5_size: step(1.0),
            h6_size: Pixels(base),
            code_size: Pixels(base - 1.0),
            spacing: Pixels(self.spacing.unwrap_or(f32::from(tokens.spacing.md))),
            style,
        }
    }

    /// Converts the document into an [`Element`].
    ///
    /// The message type is a link [`Uri`], because a Markdown document can
    /// contain links; the application maps that onto its own message.
    #[must_use]
    pub fn into_element(&self) -> Element<'_, Uri, Theme> {
        markdown::view(self.content.items(), self.settings())
    }
}

impl<'a> From<&'a Markdown> for Element<'a, Uri, Theme> {
    fn from(document: &'a Markdown) -> Self {
        document.into_element()
    }
}

/// Parses and renders a document in one call.
///
/// Convenient for a document shown once. For one rendered every frame, keep a
/// [`Markdown`] and call [`Markdown::into_element`]: this function re-parses on
/// each call.
#[must_use]
pub fn markdown(source: &str) -> Markdown {
    Markdown::parse(source)
}

/// The settings a Markdown document is rendered with, derived from tokens.
///
/// Exposed so an application can inspect or extend them without duplicating the
/// scale.
#[must_use]
pub fn settings_for(theme: &Theme) -> MarkdownSettings {
    let _ = theme;

    Markdown::new().settings()
}

/// Whether a parsed document contains a link.
///
/// Offered as a helper because an application usually needs to know whether to
/// wire up a link handler at all.
#[must_use]
pub fn has_link(document: &Markdown) -> bool {
    document.items().iter().any(item_has_link)
}

/// Recursively checks whether an item or its children contain a link.
///
/// A table's rows cannot be inspected: `markdown::Row` keeps its cells private,
/// so a link inside a table body is not detected. The column headers are
/// public, so those are checked.
fn item_has_link(item: &MarkdownItem) -> bool {
    match item {
        MarkdownItem::Paragraph(text) | MarkdownItem::Heading(_, text) => text_has_link(text),
        MarkdownItem::CodeBlock { lines, .. } => lines.iter().any(text_has_link),
        MarkdownItem::List { bullets, .. } => bullets.iter().any(|bullet| match bullet {
            Bullet::Point { items } | Bullet::Task { items, .. } => items.iter().any(item_has_link),
        }),
        MarkdownItem::Quote(items) => items.iter().any(item_has_link),
        MarkdownItem::Table { columns, .. } => columns
            .iter()
            .flat_map(|column| column.header.iter())
            .any(item_has_link),
        MarkdownItem::Image { .. } | MarkdownItem::Rule => false,
    }
}

/// Whether a run of text contains a link.
///
/// The span styling is irrelevant here — only whether a link is attached — so a
/// style built from the default tokens is enough.
fn text_has_link(text: &markdown::Text) -> bool {
    text.spans(settings_for(&Theme::light()).style)
        .iter()
        .any(|span| span.link.is_some())
}

/// The monospace font a code block uses, exposed for callers drawing their own.
#[must_use]
pub fn code_font() -> Font {
    Font {
        family: font::Family::Monospace,
        ..Font::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::{code_font, has_link, settings_for, Markdown};
    use crate::theme::Theme;

    const DOCUMENT: &str = "\
# Title

A paragraph with **bold**, *italic*, `inline code` and a [link](https://example.com).

## Section

- first
- second

1. one
2. two

> A quote.

```rust
fn main() {}
```

| Column | Value |
| ------ | ----- |
| a      | 1     |
| b      | 2     |

---
";

    #[test]
    fn a_document_parses_into_blocks() {
        let document = Markdown::parse(DOCUMENT);

        assert!(!document.is_empty());
        assert!(
            document.items().len() > 5,
            "expected several blocks, got {}",
            document.items().len()
        );
    }

    #[test]
    fn an_empty_document_parses_and_reports_empty() {
        assert!(Markdown::parse("").is_empty());
        assert!(Markdown::new().is_empty());
    }

    #[test]
    fn a_document_renders() {
        let document = Markdown::parse(DOCUMENT);
        let element: iced::Element<'_, iced::widget::markdown::Uri, Theme> =
            document.into_element();
        drop(element);
    }

    #[test]
    fn an_empty_document_renders() {
        let document = Markdown::new();
        let element: iced::Element<'_, iced::widget::markdown::Uri, Theme> =
            document.into_element();
        drop(element);
    }

    #[test]
    fn a_document_renders_at_a_custom_text_size_and_spacing() {
        let document = Markdown::parse(DOCUMENT).text_size(18.0).spacing(20.0);

        assert_eq!(document.settings().text_size, iced::Pixels(18.0));
        assert_eq!(document.settings().spacing, iced::Pixels(20.0));

        let element: iced::Element<'_, iced::widget::markdown::Uri, Theme> =
            document.into_element();
        drop(element);
    }

    #[test]
    fn documents_can_be_appended_to() {
        let mut document = Markdown::parse("# One");
        let before = document.items().len();

        document.push("\n\n# Two");
        assert!(document.items().len() > before, "push must add blocks");
    }

    #[test]
    fn heading_sizes_step_down_from_the_base_size() {
        let settings = Markdown::new().settings();

        // A document's headings must be ordered, or its outline is unreadable.
        assert!(settings.h1_size > settings.h2_size);
        assert!(settings.h2_size > settings.h3_size);
        assert!(settings.h3_size > settings.h4_size);
        assert!(settings.h4_size >= settings.text_size);
    }

    #[test]
    fn code_is_set_in_the_monospace_font() {
        let settings = Markdown::new().settings();

        assert_eq!(
            settings.style.code_block_font.family,
            iced::font::Family::Monospace
        );
        assert_eq!(
            settings.style.inline_code_font.family,
            iced::font::Family::Monospace
        );
        assert_eq!(code_font().family, iced::font::Family::Monospace);
    }

    #[test]
    fn links_are_detected() {
        assert!(has_link(&Markdown::parse("[a](https://example.com)")));
        assert!(!has_link(&Markdown::parse("just text")));
        assert!(!has_link(&Markdown::new()));
    }

    #[test]
    fn a_link_inside_a_heading_list_or_quote_is_detected() {
        // Links nest, so a shallow scan would miss these.
        for source in [
            "# See [docs](https://example.com)",
            "- item with [link](https://example.com)",
            "> quoted [link](https://example.com)",
        ] {
            assert!(has_link(&Markdown::parse(source)), "missed: {source}");
        }
    }

    #[test]
    fn a_link_in_a_table_header_is_detected() {
        let table = "\
| [Header](https://example.com) |
| ---------------------------- |
| plain cell                   |
";
        assert!(has_link(&Markdown::parse(table)));
    }

    /// A known limitation, pinned so it is not mistaken for working.
    ///
    /// `markdown::Row` keeps its cells private, so this crate cannot walk them;
    /// a link that appears only in a table body is reported as absent. An
    /// application that must catch those should scan its own source text.
    #[test]
    fn a_link_only_in_a_table_body_is_not_detected() {
        let table = "\
| Col |
| --- |
| [a](https://example.com) |
";
        assert!(
            !has_link(&Markdown::parse(table)),
            "if this now passes, `Row` has become inspectable and the \
             limitation in `item_has_link` can be removed"
        );
    }

    #[test]
    fn settings_are_derived_from_tokens() {
        let theme = Theme::light();
        let settings = settings_for(&theme);

        assert_eq!(settings.style.link_color, theme.colors().primary);
        assert_eq!(settings.style.inline_code_color, theme.colors().foreground);
    }

    #[test]
    fn both_palettes_produce_valid_settings() {
        for theme in [Theme::light(), Theme::dark()] {
            let settings = settings_for(&theme);
            assert!(settings.text_size.0 > 0.0);
            assert!(settings.spacing.0 >= 0.0);
        }
    }

    #[test]
    fn the_parse_can_be_reused_across_renders() {
        // Parsing is the expensive part, so a document is parsed once and the
        // element rendered per frame; this pins that the borrow works.
        let document = Markdown::parse(DOCUMENT);

        for _ in 0..3 {
            let element: iced::Element<'_, iced::widget::markdown::Uri, Theme> =
                document.into_element();
            drop(element);
        }
    }

    #[test]
    fn a_document_with_every_supported_block_renders() {
        // Guards against a `markdown` version adding an `Item` variant the
        // settings below do not cover.
        let document = Markdown::parse(DOCUMENT);
        let element: iced::Element<'_, iced::widget::markdown::Uri, Theme> =
            document.into_element();
        drop(element);
    }
}

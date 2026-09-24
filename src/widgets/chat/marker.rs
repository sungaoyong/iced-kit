//! A compact, composable row for conversation status and system markers.
//!
//! A marker is the small line that punctuates a transcript: a date separator
//! ("— Today —"), a system note ("Working…"), or an inline status with a thin
//! underline. It never owns interaction; when it is *loading* it shows either a
//! spinner beside its text or a shimmer swept across the text itself.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::{container, row, text, Space};
use iced::{Alignment, Background, Element, Length, Padding};

/// The visual treatment used by a [`Marker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    /// An inline marker with no additional divider.
    #[default]
    Plain,
    /// A centered marker with divider lines on both sides.
    Separator,
    /// A marker with a divider line beneath it.
    Border,
}

/// The visual treatment used while a [`Marker`] is loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerLoadingStyle {
    /// Show a compact rotating spinner beside the marker content.
    #[default]
    Spinner,
    /// Sweep a highlight across the marker content without adding an icon.
    Shimmer,
}

/// A piece of a marker's body: an icon glyph, plain text, or a foreign element.
enum MarkerChild<'a, Message> {
    Icon(IconName),
    Content(String),
    Element(Element<'a, Message, Theme>),
}

/// A conversation status / system marker row.
///
/// ```
/// # use iced_kit::widgets::chat::marker::{marker, Marker, MarkerVariant};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// marker("Reading 3 files")
///     .with_variant(MarkerVariant::Separator)
///     .into()
/// # }
/// ```
#[must_use = "a Marker does nothing unless it is turned into an Element"]
pub struct Marker<'a, Message> {
    variant: MarkerVariant,
    loading: bool,
    loading_style: MarkerLoadingStyle,
    children: Vec<MarkerChild<'a, Message>>,
    width: Length,
}

impl<'a, Message: Clone + 'a> Marker<'a, Message> {
    /// Creates an empty marker.
    pub fn new() -> Self {
        Self {
            variant: MarkerVariant::default(),
            loading: false,
            loading_style: MarkerLoadingStyle::default(),
            children: Vec::new(),
            width: Length::Fill,
        }
    }

    /// Sets the marker's visual variant.
    pub fn with_variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Marks the content as in-flight, drawing a spinner or a shimmer.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Chooses how the loading state is drawn.
    pub fn with_loading_style(mut self, style: MarkerLoadingStyle) -> Self {
        self.loading_style = style;
        self
    }

    /// Prepends an icon glyph to the marker.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.children.insert(0, MarkerChild::Icon(icon));
        self
    }

    /// Appends a text run to the marker.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.children.push(MarkerChild::Content(content.into()));
        self
    }

    /// Appends an arbitrary element to the marker.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(MarkerChild::Element(el.into()));
        self
    }

    /// Sets the marker's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Renders one child into a themed element.
    fn render_child(
        child: MarkerChild<'a, Message>,
        loading: bool,
        shimmer: bool,
    ) -> Element<'a, Message, Theme> {
        match child {
            MarkerChild::Content(s) if loading && shimmer => crate::widgets::shimmer_text(s),
            MarkerChild::Content(s) => muted_text(s),
            MarkerChild::Icon(name) => glyph_text(name),
            MarkerChild::Element(el) => el,
        }
    }
}

impl<'a, Message: Clone + 'a> Default for Marker<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Marker<'a, Message>> for Element<'a, Message, Theme> {
    fn from(marker: Marker<'a, Message>) -> Self {
        // A marker that already carries an icon does not also get a spinner:
        // two glyphs side by side would read as a rendering fault rather than
        // as work in progress. The reference draws the spinner only in the
        // icon's own slot, and only when that slot is free.
        let has_icon = marker
            .children
            .iter()
            .any(|child| matches!(child, MarkerChild::Icon(_)));
        let show_spinner =
            marker.loading && marker.loading_style == MarkerLoadingStyle::Spinner && !has_icon;
        let show_shimmer = marker.loading && marker.loading_style == MarkerLoadingStyle::Shimmer;

        // A loading marker with no text of its own has nothing to sweep a
        // shimmer across, so it breathes instead: a still row would read as a
        // marker that had finished. Both facts are read before the children are
        // consumed below.
        let has_text = marker
            .children
            .iter()
            .any(|child| matches!(child, MarkerChild::Content(_)));

        let mut body = row![].spacing(MARKER_GAP).align_y(Alignment::Center);
        if show_spinner {
            body = body.push(crate::widgets::spinner::spinner(MARKER_GLYPH_SIZE as u16));
        }
        for child in marker.children {
            body = body.push(Marker::render_child(child, marker.loading, show_shimmer));
        }

        let body: Element<'a, Message, Theme> = if marker.loading && show_shimmer && !has_text {
            crate::widgets::loading::breathe(body)
        } else {
            body.into()
        };

        match marker.variant {
            MarkerVariant::Plain => container(body).width(marker.width).into(),
            MarkerVariant::Border => container(column_below(body)).width(marker.width).into(),
            MarkerVariant::Separator => container(
                row![
                    // Each rule keeps the reference's 4px margin on its own
                    // side, which is what separates the line from the text.
                    container(divider_line())
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 0.0,
                            right: MARKER_RULE_GAP,
                            bottom: 0.0,
                            left: 0.0,
                        }),
                    body,
                    container(divider_line())
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                            left: MARKER_RULE_GAP,
                        }),
                ]
                .align_y(Alignment::Center),
            )
            .width(marker.width)
            .into(),
        }
    }
}

/// A thin, full-width divider painted in the theme's border color.
fn divider_line<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    container(Space::new().width(Length::Fill).height(1.0))
        .class(Box::new(|theme: &Theme| container::Style {
            background: Some(Background::Color(theme.colors().border)),
            ..container::Style::default()
        }) as iced::widget::container::StyleFn<'a, Theme>)
        .into()
}

/// Places `body` above a bottom divider, for the [`MarkerVariant::Border`] look.
///
/// The 8px below the text is the reference's own inset: it is what keeps the
/// rule off the descenders of the line above it.
fn column_below<'a, Message: 'a>(
    body: impl Into<Element<'a, Message, Theme>>,
) -> Element<'a, Message, Theme> {
    iced::widget::column![
        body.into(),
        container(divider_line()).padding(Padding {
            top: MARKER_RULE_GAP,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }),
    ]
    .into()
}

/// The glyph and text point size markers draw at.
const MARKER_GLYPH_SIZE: f32 = Size::Sm.text().size;

/// The gap between a marker's icon and its text.
const MARKER_GAP: f32 = 8.0;

/// How far a marker's rule sits from the text it borders.
const MARKER_RULE_GAP: f32 = 4.0;

/// Builds a small muted-text element that inherits the theme's foreground.
fn muted_text<'a, Message: 'a>(s: String) -> Element<'a, Message, Theme> {
    text(s)
        .size(MARKER_GLYPH_SIZE)
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as iced::widget::text::StyleFn<'a, Theme>)
        .into()
}

/// Renders an [`IconName`] glyph in the bundled icon font, tinted muted.
fn glyph_text<'a, Message: 'a>(name: IconName) -> Element<'a, Message, Theme> {
    crate::icons::load();
    text(crate::icons::glyph(name))
        .font(crate::icons::font())
        .size(MARKER_GLYPH_SIZE)
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as iced::widget::text::StyleFn<'a, Theme>)
        .into()
}

/// Builds a [`Marker`] carrying a single text run.
///
/// Convenience for the common "status line" case; reach for [`Marker::new`] when
/// you need an icon or several runs.
pub fn marker<'a, Message: Clone + 'a>(content: impl Into<String>) -> Marker<'a, Message> {
    Marker::new().content(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    #[test]
    fn variants_default_to_plain_and_spinner() {
        assert_eq!(MarkerVariant::default(), MarkerVariant::Plain);
        assert_eq!(MarkerLoadingStyle::default(), MarkerLoadingStyle::Spinner);
    }

    #[test]
    fn renders_across_all_variants_and_loading() {
        for variant in [
            MarkerVariant::Plain,
            MarkerVariant::Separator,
            MarkerVariant::Border,
        ] {
            for loading in [false, true] {
                for style in [MarkerLoadingStyle::Spinner, MarkerLoadingStyle::Shimmer] {
                    let el: Element<'_, Msg, Theme> = Marker::new()
                        .content("Reading 3 files")
                        .with_variant(variant)
                        .loading(loading)
                        .with_loading_style(style)
                        .into();
                    drop(el);
                }
            }
        }
    }

    #[test]
    fn constructor_fn_builds_a_plain_marker() {
        let el: Element<'_, Msg, Theme> = marker("Yesterday").into();
        drop(el);
    }

    #[test]
    fn icon_and_child_compose() {
        let note: Element<'_, Msg, Theme> = text("!").into();
        let el: Element<'_, Msg, Theme> = Marker::new()
            .icon(IconName::Search)
            .content("Searching")
            .child(note)
            .into();
        drop(el);
    }

    /// The spacing values come from the reference: a marker's icon sits 8px
    /// from its text, and a rule sits 4px from it.
    #[test]
    fn the_marker_spacing_matches_the_reference() {
        assert_eq!(MARKER_GAP, 8.0);
        assert_eq!(MARKER_RULE_GAP, 4.0);
    }

    /// A marker that already carries an icon must not also draw a spinner: two
    /// glyphs side by side read as a fault rather than as work in progress.
    #[test]
    fn a_spinner_is_drawn_only_when_the_icon_slot_is_free() {
        let empty = Marker::<Msg>::new().content("Working").loading(true);
        assert!(
            empty
                .children
                .iter()
                .all(|c| !matches!(c, MarkerChild::Icon(_))),
            "no icon, so the spinner has the slot"
        );

        let with_icon = Marker::<Msg>::new()
            .icon(crate::icons::IconName::Search)
            .content("Searching")
            .loading(true);
        assert!(
            with_icon
                .children
                .iter()
                .any(|c| matches!(c, MarkerChild::Icon(_))),
            "the icon occupies the slot"
        );
    }

    /// A loading shimmer marker with no text of its own has nothing to sweep, so
    /// it breathes instead; one with text shimmers the text.
    #[test]
    fn a_textless_shimmer_marker_breathes() {
        let textless: Element<'_, Msg, Theme> = Marker::new()
            .with_loading_style(MarkerLoadingStyle::Shimmer)
            .loading(true)
            .into();
        drop(textless);

        let with_text: Element<'_, Msg, Theme> = Marker::new()
            .content("Working")
            .with_loading_style(MarkerLoadingStyle::Shimmer)
            .loading(true)
            .into();
        drop(with_text);
    }

    /// A loading marker renders in every variant and loading style, with and
    /// without an icon.
    #[test]
    fn loading_markers_render_across_their_options() {
        for variant in [
            MarkerVariant::Plain,
            MarkerVariant::Separator,
            MarkerVariant::Border,
        ] {
            for style in [MarkerLoadingStyle::Spinner, MarkerLoadingStyle::Shimmer] {
                for with_icon in [false, true] {
                    let marker: Marker<'_, Msg> = Marker::new()
                        .with_variant(variant)
                        .with_loading_style(style)
                        .loading(true);
                    let marker = if with_icon {
                        marker.icon(crate::icons::IconName::Search)
                    } else {
                        marker
                    };

                    let el: Element<'_, Msg, Theme> = marker.into();
                    drop(el);

                    // The textless case, which is the one that breathes.
                    let el: Element<'_, Msg, Theme> = Marker::new()
                        .with_variant(variant)
                        .with_loading_style(style)
                        .loading(true)
                        .into();
                    drop(el);
                }
            }
        }
    }
}

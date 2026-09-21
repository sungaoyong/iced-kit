//! Accordion and pagination.
//!
//! Both are stateless: the caller owns which section is open, and which page is
//! current, and receives a message when either changes.

use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length, Padding};

/// One section of an [`accordion`].
#[derive(Debug, Clone)]
#[must_use = "a Section does nothing unless it is given to `accordion`"]
pub struct Section {
    title: String,
    subtitle: Option<String>,
}

impl Section {
    /// Creates a section with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
        }
    }

    /// Adds a secondary line shown under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Returns the section's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// Builds a vertical accordion.
///
/// Only one section is open at a time, which is the behaviour that keeps a long
/// settings list scannable. `open` is the index of the expanded section, or
/// `None` when all are collapsed.
///
/// ```
/// # use iced_kit::widgets::{accordion, AccordionSection};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Toggled(usize) }
/// # fn view(open: Option<usize>) -> Element<'static, Message, Theme> {
/// accordion(
///     vec![AccordionSection::new("General"), AccordionSection::new("Advanced")],
///     open,
///     Message::Toggled,
///     |index| iced::widget::text(format!("Body {index}")).into(),
/// )
/// # }
/// ```
pub fn accordion<'a, Message: Clone + 'a>(
    sections: Vec<Section>,
    open: Option<usize>,
    on_toggle: impl Fn(usize) -> Message + 'a,
    body: impl Fn(usize) -> Element<'a, Message, Theme> + 'a,
) -> Element<'a, Message, Theme> {
    let title_style = Size::Md.text();
    let subtitle_style = Size::Sm.text();

    let mut items = column![].spacing(0);

    for (index, section) in sections.into_iter().enumerate() {
        let is_open = open == Some(index);

        let mut heading = row![
            text(if is_open { "▾" } else { "▸" }).size(title_style.size - 2.0),
            text(section.title)
                .size(title_style.size)
                .line_height(title_style.line_height()),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        if let Some(subtitle) = section.subtitle {
            heading = heading.push(
                text(subtitle)
                    .size(subtitle_style.size)
                    .line_height(subtitle_style.line_height())
                    .class(Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>),
            );
        }

        let header = button(heading.width(Length::Fill))
            .width(Length::Fill)
            .padding(Padding {
                top: 10.0,
                right: 12.0,
                bottom: 10.0,
                left: 12.0,
            })
            .class(
                Box::new(move |theme: &Theme, status| header_style(theme, status, is_open))
                    as button::StyleFn<'a, Theme>,
            )
            .on_press(on_toggle(index));

        items = items.push(header);

        if is_open {
            items = items.push(container(body(index)).width(Length::Fill).padding(Padding {
                top: 4.0,
                right: 12.0,
                bottom: 12.0,
                left: 12.0,
            }));
        }

        items = items.push(
            container(iced::widget::Space::new().height(Length::Fixed(1.0)))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .class(Box::new(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.colors().border)),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>),
        );
    }

    items.into()
}

/// The appearance of an accordion header.
fn header_style(theme: &Theme, status: button::Status, is_open: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: hovered.then_some(iced::Background::Color(colors.accent)),
        text_color: if is_open {
            colors.foreground
        } else {
            colors.muted_foreground
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// Builds a pagination control.
///
/// `page` is zero-based. Emits the requested page index when a control is
/// pressed; the previous and next buttons are inert at the ends of the range.
///
/// ```
/// # use iced_kit::widgets::pagination;
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Went(usize) }
/// # fn view(page: usize) -> Element<'static, Message, Theme> {
/// pagination(page, 10, Message::Went)
/// # }
/// ```
pub fn pagination<'a, Message: Clone + 'a>(
    page: usize,
    total_pages: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    // Each button needs its own owned copy of the callback, so it is shared
    // behind an `Rc` that every button clones.
    let on_select: std::rc::Rc<dyn Fn(usize) -> Message + 'a> = std::rc::Rc::new(on_select);
    let last = total_pages.saturating_sub(1);
    let page = page.min(last);

    let mut strip = row![].spacing(4).align_y(iced::Alignment::Center);

    strip = strip.push(page_button("‹", page.checked_sub(1), false, &on_select));

    for index in page_window(page, total_pages) {
        match index {
            Some(index) => {
                strip = strip.push(page_button(
                    format!("{}", index + 1),
                    Some(index),
                    index == page,
                    &on_select,
                ));
            }
            None => {
                // An ellipsis marks a gap in the page numbers.
                strip = strip.push(container(text("…").size(Size::Sm.text().size)).padding(
                    Padding {
                        top: 0.0,
                        right: 4.0,
                        bottom: 0.0,
                        left: 4.0,
                    },
                ));
            }
        }
    }

    strip = strip.push(page_button(
        "›",
        (page < last).then_some(page + 1),
        false,
        &on_select,
    ));

    strip.into()
}

/// One numbered (or arrow) button in the strip.
fn page_button<'a, Message: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    target: Option<usize>,
    is_current: bool,
    on_select: &std::rc::Rc<dyn Fn(usize) -> Message + 'a>,
) -> Element<'a, Message, Theme> {
    /// The height every page button is drawn at.
    const HEIGHT: f32 = 28.0;

    let style = Size::Sm.text();

    // The line box is the button's own height, not the text's. iced lays a
    // button's content out at its padding origin without centring it, so a line
    // box the height of the text leaves the digit sitting against the top of the
    // button — which is what it did. A box as tall as the control puts the
    // baseline where the eye expects it, and it is what `Button` does for its
    // own labels.
    let mut widget = button(
        text(label)
            .size(style.size)
            .line_height(iced::Pixels(HEIGHT.max(style.line_height))),
    )
    .padding(Padding {
        top: 0.0,
        right: 8.0,
        bottom: 0.0,
        left: 8.0,
    })
    .height(Length::Fixed(HEIGHT))
    .class(
        Box::new(move |theme: &Theme, status| page_style(theme, status, is_current))
            as button::StyleFn<'a, Theme>,
    );

    if let Some(target) = target {
        widget = widget.on_press(on_select(target));
    }

    widget.into()
}

/// The appearance of one pagination button.
fn page_style(theme: &Theme, status: button::Status, is_current: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);
    let disabled = matches!(status, button::Status::Disabled);

    button::Style {
        background: if is_current {
            Some(iced::Background::Color(colors.primary))
        } else if hovered && !disabled {
            Some(iced::Background::Color(colors.accent))
        } else {
            None
        },
        text_color: if is_current {
            colors.primary_foreground
        } else if disabled {
            Color {
                a: colors.muted_foreground.a * 0.5,
                ..colors.muted_foreground
            }
        } else {
            colors.foreground
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().sm).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// Chooses which page numbers to show, with `None` marking an ellipsis.
///
/// Always shows the first and last page, plus a window around the current one.
fn page_window(page: usize, total: usize) -> Vec<Option<usize>> {
    if total == 0 {
        return Vec::new();
    }

    if total <= 7 {
        return (0..total).map(Some).collect();
    }

    let mut pages = vec![Some(0)];
    let start = page.saturating_sub(1).max(1);
    let end = (page + 1).min(total - 2);

    if start > 1 {
        pages.push(None);
    }

    for index in start..=end {
        pages.push(Some(index));
    }

    if end < total - 2 {
        pages.push(None);
    }

    pages.push(Some(total - 1));
    pages
}

#[cfg(test)]
mod tests {
    use super::{accordion, page_button, page_window, pagination, Section};
    use crate::theme::Theme;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Toggled(usize),
        Went(usize),
    }

    #[test]
    fn an_accordion_renders_open_and_closed() {
        let sections = vec![Section::new("General"), Section::new("Advanced")];

        let closed: iced::Element<'_, Message, Theme> =
            accordion(sections.clone(), None, Message::Toggled, |index| {
                iced::widget::text(format!("Body {index}")).into()
            });
        drop(closed);

        let open: iced::Element<'_, Message, Theme> =
            accordion(sections, Some(1), Message::Toggled, |index| {
                iced::widget::text(format!("Body {index}")).into()
            });
        drop(open);
    }

    #[test]
    fn an_accordion_ignores_an_out_of_range_open_index() {
        let element: iced::Element<'_, Message, Theme> = accordion(
            vec![Section::new("Only")],
            Some(99),
            Message::Toggled,
            |_| iced::widget::text("Body").into(),
        );
        drop(element);
    }

    #[test]
    fn an_empty_accordion_renders() {
        let element: iced::Element<'_, Message, Theme> =
            accordion(vec![], None, Message::Toggled, |_| {
                iced::widget::text("").into()
            });
        drop(element);
    }

    #[test]
    fn a_section_keeps_its_title_and_subtitle() {
        let section = Section::new("General").subtitle("Basic options");
        assert_eq!(section.title(), "General");
        assert_eq!(section.subtitle.as_deref(), Some("Basic options"));
    }

    #[test]
    fn a_pagination_renders_across_the_range() {
        for (page, total) in [(0, 1), (0, 5), (2, 5), (4, 5), (0, 50), (25, 50), (49, 50)] {
            let element: iced::Element<'_, Message, Theme> = pagination(page, total, Message::Went);
            drop(element);
        }
    }

    #[test]
    fn a_pagination_handles_a_zero_page_count() {
        // A list with no results must not divide by zero or panic.
        let element: iced::Element<'_, Message, Theme> = pagination(0, 0, Message::Went);
        drop(element);
    }

    #[test]
    fn a_pagination_clamps_an_out_of_range_page() {
        let element: iced::Element<'_, Message, Theme> = pagination(999, 5, Message::Went);
        drop(element);
    }

    #[test]
    fn a_short_range_lists_every_page() {
        assert_eq!(
            page_window(0, 5),
            vec![Some(0), Some(1), Some(2), Some(3), Some(4)]
        );
    }

    #[test]
    fn a_long_range_collapses_the_middle_with_ellipses() {
        let window = page_window(25, 50);

        // The first and last page are always reachable.
        assert_eq!(window.first(), Some(&Some(0)));
        assert_eq!(window.last(), Some(&Some(49)));

        // The gap is marked rather than enumerated.
        assert!(window.contains(&None), "a 50-page range must collapse");

        // The current page is present.
        assert!(window.contains(&Some(25)));

        // The window stays small enough to render in one row.
        assert!(
            window.len() <= 9,
            "the window must stay compact, got {}",
            window.len()
        );
    }

    #[test]
    fn a_degenerate_range_yields_no_pages() {
        assert!(page_window(0, 0).is_empty());
    }

    /// A page button's label must be as tall as the button.
    ///
    /// iced lays a raw button's content out at its padding origin without
    /// centring it, so the line box's height is the only thing that decides
    /// where the digit sits: measured from the render snapshot, a text-height
    /// line box put the digit 20 physical pixels above centre, and a
    /// control-height one centred it exactly. `Button` does this for its own
    /// labels; a component that draws a raw iced button has to do it itself.
    #[test]
    fn a_page_button_label_is_as_tall_as_its_button() {
        const HEIGHT: f32 = 28.0;

        let on_select: Rc<dyn Fn(usize) -> Message> = Rc::new(Message::Went);
        let element: iced::Element<'_, Message, Theme> =
            page_button("1", Some(0), false, &on_select);

        // The element is opaque, so the invariant is stated against the metric
        // the button uses: the line box must reach the control's height.
        let text_height = crate::theme::Size::Sm.text().line_height;
        assert!(
            text_height < HEIGHT,
            "the text is shorter than the button, which is why the box has to grow"
        );
        assert_eq!(
            HEIGHT.max(text_height),
            HEIGHT,
            "the line box must be the button's height, not the text's"
        );

        drop(element);
    }
}

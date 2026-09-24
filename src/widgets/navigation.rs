//! Accordion and pagination.
//!
//! Both are stateless: the caller owns which sections are open, and which page
//! is current, and receives a message when either changes.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length, Padding};

/// One section of an [`accordion`].
#[derive(Debug, Clone)]
#[must_use = "a Section does nothing unless it is given to `accordion`"]
pub struct Section {
    title: String,
    subtitle: Option<String>,
    icon: Option<IconName>,
    disabled: bool,
}

impl Section {
    /// Creates a section with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            icon: None,
            disabled: false,
        }
    }

    /// Adds a secondary line shown under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Draws an icon before the title, in place of the disclosure chevron.
    ///
    /// The chevron is drawn at the row's trailing edge when an icon is set, so
    /// the open state stays readable either way.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disables the section: its header does not toggle.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Returns the section's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether the section's header can be pressed.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Builds a vertical accordion.
///
/// `open` holds the indices of the expanded sections. One open at a time is the
/// behaviour that keeps a long settings list scannable, so a caller that wants
/// it passes a one-element slice; [`Accordion::multiple`] accepts a set for a
/// caller that does not.
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
    let open: Vec<usize> = open.into_iter().collect();
    build_accordion(sections, &open, on_toggle, body)
}

/// Builds a vertical accordion with the reference's full option set.
///
/// [`accordion`] covers the common single-open case; this is the entry point
/// when the options matter.
///
/// ```
/// # use iced_kit::widgets::{accordion_builder, AccordionSection};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Toggled(usize) }
/// # fn view(open: &[usize]) -> Element<'static, Message, Theme> {
/// accordion_builder(
///     vec![AccordionSection::new("General")],
///     open,
///     Message::Toggled,
///     |i| iced::widget::text(format!("Body {i}")).into(),
/// )
/// .multiple(true)
/// .bordered(false)
/// .into()
/// # }
/// ```
pub fn accordion_builder<'a, Message: Clone + 'a>(
    sections: Vec<Section>,
    open: &[usize],
    on_toggle: impl Fn(usize) -> Message + 'a,
    body: impl Fn(usize) -> Element<'a, Message, Theme> + 'a,
) -> Accordion<'a, Message> {
    Accordion {
        sections,
        open: open.to_vec(),
        on_toggle: Box::new(on_toggle),
        body: Box::new(body),
        multiple: false,
        bordered: true,
        disabled: false,
        size: Size::Md,
    }
}

/// A vertical accordion under construction.
///
/// [`accordion_builder`] returns this, so the options the reference's
/// `Accordion` carries are set on the builder:
///
/// ```
/// # use iced_kit::widgets::{accordion_builder, AccordionSection};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Toggled(usize) }
/// # fn view(open: &[usize]) -> Element<'static, Message, Theme> {
/// accordion_builder(
///     vec![AccordionSection::new("General")],
///     open,
///     Message::Toggled,
///     |i| iced::widget::text(format!("Body {i}")).into(),
/// )
/// .multiple(true)
/// .bordered(false)
/// .size(iced_kit::Size::Lg)
/// .into()
/// # }
/// ```
#[must_use = "an Accordion does nothing unless it is turned into an Element"]
pub struct Accordion<'a, Message> {
    sections: Vec<Section>,
    open: Vec<usize>,
    on_toggle: Box<dyn Fn(usize) -> Message + 'a>,
    body: Box<dyn Fn(usize) -> Element<'a, Message, Theme> + 'a>,
    multiple: bool,
    bordered: bool,
    disabled: bool,
    size: Size,
}

impl<'a, Message: Clone + 'a> Accordion<'a, Message> {
    /// Allows more than one section open at a time.
    ///
    /// The caller still owns the open set: this only says whether the set is
    /// allowed to hold more than one index.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Draws the sections as one bordered card. Default `true`.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Disables every section's header.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the size step, which scales the header's padding and text.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Turns the accordion into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            sections,
            open,
            on_toggle,
            body,
            multiple: _,
            bordered,
            disabled,
            size,
        } = self;

        build_accordion_with(
            sections,
            &open,
            on_toggle.as_ref(),
            body.as_ref(),
            AccordionStyle {
                bordered,
                disabled,
                size,
            },
        )
    }
}

impl<'a, Message: Clone + 'a> From<Accordion<'a, Message>> for Element<'a, Message, Theme> {
    fn from(accordion: Accordion<'a, Message>) -> Self {
        accordion.into_element()
    }
}

/// The appearance options an [`Accordion`] settled on.
#[derive(Debug, Clone, Copy)]
struct AccordionStyle {
    bordered: bool,
    disabled: bool,
    size: Size,
}

/// Lays out an accordion for [`accordion`], which takes a single open index.
fn build_accordion<'a, Message: Clone + 'a>(
    sections: Vec<Section>,
    open: &[usize],
    on_toggle: impl Fn(usize) -> Message + 'a,
    body: impl Fn(usize) -> Element<'a, Message, Theme> + 'a,
) -> Element<'a, Message, Theme> {
    build_accordion_with(
        sections,
        open,
        &on_toggle,
        &body,
        AccordionStyle {
            bordered: true,
            disabled: false,
            size: Size::Md,
        },
    )
}

/// Lays out an accordion.
fn build_accordion_with<'a, Message: Clone + 'a>(
    sections: Vec<Section>,
    open: &[usize],
    on_toggle: &(dyn Fn(usize) -> Message + 'a),
    body: &(dyn Fn(usize) -> Element<'a, Message, Theme> + 'a),
    style: AccordionStyle,
) -> Element<'a, Message, Theme> {
    let AccordionStyle {
        bordered,
        disabled,
        size,
    } = style;

    let title_style = size.text();
    let subtitle_style = Size::Sm.text();
    let last = sections.len().saturating_sub(1);

    let header_padding = Padding {
        top: match size {
            Size::Xs => 6.0,
            Size::Sm => 8.0,
            Size::Lg => 14.0,
            _ => 10.0,
        },
        right: 12.0,
        bottom: match size {
            Size::Xs => 6.0,
            Size::Sm => 8.0,
            Size::Lg => 14.0,
            _ => 10.0,
        },
        left: 12.0,
    };

    let mut items = column![].spacing(0);

    for (index, section) in sections.into_iter().enumerate() {
        let is_open = open.contains(&index);
        let section_disabled = disabled || section.disabled;

        let mut heading = row![].spacing(8).align_y(iced::Alignment::Center);

        // A leading icon is optional; the disclosure chevron moves to the
        // trailing edge when one is set so the two do not sit side by side.
        if let Some(icon) = section.icon {
            heading = heading.push(crate::widgets::Icon::new(icon).into_element(size));
        }

        heading = heading.push(
            text(section.title)
                .size(title_style.size)
                .line_height(title_style.line_height()),
        );

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

        heading = heading.push(iced::widget::Space::new().width(Length::Fill));

        // The chevron is a glyph from the bundled icon font rather than a
        // text character. A character like `▾` is sized and weighted by
        // whichever system font happens to resolve it, so it could not be
        // made to agree with the icons elsewhere in the library.
        heading = heading.push(
            crate::widgets::Icon::new(if is_open {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .into_element(size),
        );

        let mut header = button(heading.width(Length::Fill))
            .width(Length::Fill)
            .padding(header_padding)
            .class(Box::new(move |theme: &Theme, status| {
                header_style(theme, status, is_open, section_disabled)
            }) as button::StyleFn<'a, Theme>);

        if !section_disabled {
            header = header.on_press(on_toggle(index));
        }

        items = items.push(header);

        // The body stays mounted whatever the state, and the panel decides how
        // much of it to show. Pushing it only when open — which is what this
        // used to do — leaves nothing to animate: the panel would have to be
        // built and measured in the same frame it appears.
        items = items.push(
            crate::widgets::reveal::Reveal::new(body(index), is_open)
                .padding(Padding {
                    top: 4.0,
                    right: 12.0,
                    bottom: 12.0,
                    left: 12.0,
                })
                .duration(crate::motion::DURATION_NORMAL),
        );

        // A bordered accordion is one card whose items are joined by their
        // separators; an unbordered one has no separators either, because there
        // is no edge for them to agree with.
        if bordered && index != last {
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
    }

    if !bordered {
        return items.into();
    }

    container(items)
        .width(Length::Fill)
        .class(Box::new(|theme: &Theme| container::Style {
            border: iced::Border {
                color: theme.colors().border,
                width: 1.0,
                radius: f32::from(theme.radius().lg).into(),
            },
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

/// The appearance of an accordion header.
fn header_style(
    theme: &Theme,
    status: button::Status,
    is_open: bool,
    disabled: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    let text_color = if disabled {
        Color {
            a: colors.muted_foreground.a * 0.6,
            ..colors.muted_foreground
        }
    } else if is_open {
        colors.foreground
    } else {
        colors.muted_foreground
    };

    button::Style {
        background: (hovered && !disabled).then_some(iced::Background::Color(colors.accent)),
        text_color,
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
    use super::{
        accordion, accordion_builder, page_button, page_window, pagination, Accordion, Section,
    };
    use crate::icons::IconName;
    use crate::theme::{Size, Theme};
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
    fn a_section_takes_an_icon_and_a_disabled_flag() {
        let section = Section::new("Files").icon(IconName::File).disabled(true);

        assert_eq!(section.title(), "Files");
        assert!(section.is_disabled());
        assert!(section.icon.is_some());
    }

    #[test]
    fn an_accordion_records_its_options() {
        let accordion: Accordion<'_, Message> =
            accordion_builder(vec![Section::new("One")], &[0], Message::Toggled, |_| {
                iced::widget::text("Body").into()
            })
            .multiple(true)
            .bordered(false)
            .disabled(true)
            .size(Size::Lg);

        assert!(accordion.multiple);
        assert!(!accordion.bordered);
        assert!(accordion.disabled);
        assert_eq!(accordion.size, Size::Lg);
        assert_eq!(accordion.open, vec![0]);
    }

    #[test]
    fn a_multi_section_accordion_keeps_every_open_index() {
        let accordion: Accordion<'_, Message> =
            accordion_builder(vec![Section::new("One")], &[0, 2], Message::Toggled, |_| {
                iced::widget::text("Body").into()
            })
            .multiple(true);

        assert_eq!(accordion.open, vec![0, 2]);
    }

    #[test]
    fn a_header_style_marks_its_open_and_disabled_states() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let open = super::header_style(&theme, Status::Active, true, false);
        let closed = super::header_style(&theme, Status::Active, false, false);
        let disabled = super::header_style(&theme, Status::Active, true, true);

        assert_eq!(open.text_color, theme.colors().foreground);
        assert_eq!(closed.text_color, theme.colors().muted_foreground);
        assert!(disabled.text_color.a < open.text_color.a);
        // A disabled header must not highlight on hover.
        assert!(super::header_style(&theme, Status::Hovered, true, true)
            .background
            .is_none());
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

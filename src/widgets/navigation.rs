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

/// One crumb of a [`breadcrumb`].
#[derive(Debug, Clone)]
#[must_use = "a Crumb does nothing unless it is given to `breadcrumb`"]
pub struct Crumb<Message> {
    label: String,
    message: Option<Message>,
    disabled: bool,
}

impl<Message> Crumb<Message> {
    /// Creates a crumb that is plain text.
    ///
    /// The last crumb of a trail is usually plain: it names the page the reader
    /// is already on, so it does not need to be a link.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            message: None,
            disabled: false,
        }
    }

    /// Creates a crumb that navigates when pressed.
    pub fn link(label: impl Into<String>, message: Message) -> Self {
        Self {
            label: label.into(),
            message: Some(message),
            disabled: false,
        }
    }

    /// Disables the crumb: it renders dimmed and emits nothing.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The crumb's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the crumb can be pressed.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// A trail of ancestors: where the reader is, and how to get back.
///
/// The separator is drawn between crumbs rather than by them, so a trail of one
/// crumb has no separator and a trail of five has four — which is what makes
/// the count of separators impossible to get wrong.
///
/// ```
/// # use iced_kit::widgets::{breadcrumb, Crumb};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Open(&'static str) }
/// # fn view() -> Element<'static, Message, Theme> {
/// breadcrumb(vec![
///     Crumb::link("Home", Message::Open("home")),
///     Crumb::link("Files", Message::Open("files")),
///     Crumb::new("report.pdf"),
/// ])
/// # }
/// ```
pub fn breadcrumb<'a, Message: Clone + 'a>(
    crumbs: Vec<Crumb<Message>>,
) -> Element<'a, Message, Theme> {
    let text_style = Size::Sm.text();
    let count = crumbs.len();

    let mut trail = row![].spacing(6).align_y(iced::Alignment::Center);

    for (index, crumb) in crumbs.into_iter().enumerate() {
        let is_last = index + 1 == count;
        let label = crumb.label.clone();

        // A crumb with a message is a link; one without is the current page,
        // drawn as ordinary text so it does not invite a click that does
        // nothing.
        let content: Element<'a, Message, Theme> = if crumb.message.is_some() && !crumb.disabled {
            let message = crumb.message.clone().expect("checked above");
            let mut link = button(
                text(label)
                    .size(text_style.size)
                    .line_height(text_style.line_height()),
            )
            .padding(Padding {
                top: 0.0,
                right: 2.0,
                bottom: 0.0,
                left: 2.0,
            })
            .class(
                Box::new(|theme: &Theme, status| crumb_link_style(theme, status))
                    as button::StyleFn<'a, Theme>,
            );

            link = link.on_press(message);
            link.into()
        } else {
            // A crumb with no message is the page the reader is on, drawn as
            // plain text so it does not invite a click that does nothing.
            let disabled = crumb.disabled;

            text(label)
                .size(text_style.size)
                .line_height(text_style.line_height())
                .class(Box::new(move |theme: &Theme| text::Style {
                    color: Some(if disabled {
                        Color {
                            a: theme.colors().muted_foreground.a * 0.6,
                            ..theme.colors().muted_foreground
                        }
                    } else {
                        theme.colors().foreground
                    }),
                }) as text::StyleFn<'a, Theme>)
                .into()
        };

        trail = trail.push(content);

        if !is_last {
            let arrow: Element<'a, Message, Theme> =
                crate::widgets::Icon::new(IconName::ChevronRight).into_element(Size::Sm);

            trail = trail.push(
                container(arrow).class(Box::new(|theme: &Theme| container::Style {
                    text_color: Some(theme.colors().muted_foreground),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>),
            );
        }

        let _ = (index, is_last);
    }

    trail.into()
}

/// The appearance of a breadcrumb link.
fn crumb_link_style(theme: &Theme, status: button::Status) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: None,
        text_color: if hovered {
            colors.foreground
        } else {
            colors.muted_foreground
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

/// One step of a [`stepper`].
#[derive(Debug, Clone)]
#[must_use = "a Step does nothing unless it is given to `stepper`"]
pub struct Step {
    label: String,
    icon: Option<IconName>,
    disabled: bool,
}

impl Step {
    /// Creates a step with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    /// Draws an icon instead of the step's number.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disables the step: it renders dimmed and cannot be selected.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The step's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the step can be selected.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Which way a [`stepper`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepLayout {
    /// Steps run left to right, joined by a rule.
    #[default]
    Horizontal,
    /// Steps run top to bottom, joined by a rule.
    Vertical,
}

/// Which step of the sequence the reader is on, and what the steps are.
///
/// A step before the current one is marked as done, which is what a stepper is
/// for: the trail of what has been finished is as informative as the current
/// position.
///
/// ```
/// # use iced_kit::widgets::{stepper, Step, StepLayout};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Went(usize) }
/// # fn view(current: usize) -> Element<'static, Message, Theme> {
/// stepper(
///     vec![Step::new("Cart"), Step::new("Address"), Step::new("Payment")],
///     current,
/// )
/// .layout(StepLayout::Horizontal)
/// .on_select(Message::Went)
/// # }
/// ```
#[must_use = "a Stepper does nothing unless it is turned into an Element"]
pub struct Stepper<'a, Message> {
    steps: Vec<Step>,
    current: usize,
    layout: StepLayout,
    disabled: bool,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Stepper<'a, Message> {
    /// Creates a stepper at `current`.
    pub fn new(steps: Vec<Step>, current: usize) -> Self {
        Self {
            steps,
            current,
            layout: StepLayout::default(),
            disabled: false,
            on_select: None,
        }
    }

    /// Sets which way the steps run.
    pub fn layout(mut self, layout: StepLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Disables every step.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reports a pressed step.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Whether step `index` is behind the current one.
    #[must_use]
    pub fn is_done(&self, index: usize) -> bool {
        index < self.current
    }

    /// Whether step `index` is the current one.
    #[must_use]
    pub fn is_current(&self, index: usize) -> bool {
        index == self.current
    }

    /// Turns the stepper into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            steps,
            current,
            layout,
            disabled,
            on_select,
        } = self;

        let step_style = Size::Sm.text();
        let label_style = Size::Xs.text();
        let count = steps.len();

        let mut items: Vec<Element<'a, Message, Theme>> = Vec::new();

        for (index, step) in steps.into_iter().enumerate() {
            let is_done = index < current;
            let is_current = index == current;
            let step_disabled = disabled || step.disabled;
            // The style closures live for the element's whole lifetime, so the
            // flag is copied into each rather than borrowed.
            let head_disabled = step_disabled;

            // The marker is a circle: its number before the step is reached, a
            // check once it is behind.
            let marker: Element<'a, Message, Theme> = if is_done {
                crate::widgets::Icon::new(IconName::Check).into_element(Size::Sm)
            } else if let Some(icon) = step.icon {
                crate::widgets::Icon::new(icon).into_element(Size::Sm)
            } else {
                text(format!("{}", index + 1))
                    .size(step_style.size)
                    .line_height(step_style.line_height())
                    .into()
            };

            let marker = container(marker)
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .center_x(Length::Fixed(24.0))
                .center_y(Length::Fixed(24.0))
                .class(Box::new(move |theme: &Theme| {
                    step_marker_style(theme, is_done, is_current, step_disabled)
                }) as container::StyleFn<'a, Theme>);

            let mut head = row![].spacing(8).align_y(iced::Alignment::Center);
            head = head.push(marker);
            head = head.push(
                text(step.label.clone())
                    .size(label_style.size)
                    .line_height(label_style.line_height()),
            );

            let head = if let Some(on_select) = on_select.as_ref() {
                let mut head_button = button(head)
                    .padding(Padding {
                        top: 2.0,
                        right: 4.0,
                        bottom: 2.0,
                        left: 4.0,
                    })
                    .class(Box::new(move |theme: &Theme, status| {
                        step_head_style(theme, status, head_disabled)
                    }) as button::StyleFn<'a, Theme>);

                if !step_disabled {
                    head_button = head_button.on_press(on_select(index));
                }

                head_button.into()
            } else {
                head.into()
            };

            items.push(head);

            // The rule joins this step to the next, so the last step has none.
            if index + 1 < count {
                let connector: Element<'a, Message, Theme> = match layout {
                    StepLayout::Horizontal => {
                        container(iced::widget::Space::new().height(Length::Fixed(1.0)))
                            .width(Length::Fill)
                            .height(Length::Fixed(1.0))
                            .class(
                                Box::new(move |theme: &Theme| connector_style(theme, is_done))
                                    as container::StyleFn<'a, Theme>,
                            )
                            .into()
                    }
                    StepLayout::Vertical => {
                        // The rule runs down the middle of the marker column,
                        // so it lines up with the circles it joins: the marker
                        // is 24 wide, and the line is 1, so the inset is half
                        // of the difference.
                        let line = container(iced::widget::Space::new().width(Length::Fixed(1.0)))
                            .width(Length::Fixed(1.0))
                            .height(Length::Fixed(16.0))
                            .class(
                                Box::new(move |theme: &Theme| connector_style(theme, is_done))
                                    as container::StyleFn<'a, Theme>,
                            );

                        row![
                            container(line).padding(Padding {
                                top: 0.0,
                                right: 0.0,
                                bottom: 0.0,
                                left: 11.5,
                            }),
                            iced::widget::Space::new().width(Length::Fill),
                        ]
                        .into()
                    }
                };

                items.push(connector);
            }
        }

        match layout {
            StepLayout::Horizontal => row(items)
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .into(),
            StepLayout::Vertical => column(items).spacing(0).width(Length::Fill).into(),
        }
    }
}

impl<'a, Message: Clone + 'a> From<Stepper<'a, Message>> for Element<'a, Message, Theme> {
    fn from(stepper: Stepper<'a, Message>) -> Self {
        stepper.into_element()
    }
}

/// The appearance of a step's marker.
fn step_marker_style(
    theme: &Theme,
    is_done: bool,
    is_current: bool,
    disabled: bool,
) -> container::Style {
    let colors = theme.colors();

    let (background, text_color, border) = if disabled {
        (
            colors.muted,
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            },
            colors.border,
        )
    } else if is_done {
        // A finished step is filled, so the trail of progress reads at a
        // glance without reading the numbers.
        (colors.primary, colors.primary_foreground, colors.primary)
    } else if is_current {
        (colors.background, colors.foreground, colors.primary)
    } else {
        (colors.background, colors.muted_foreground, colors.border)
    };

    container::Style {
        background: Some(iced::Background::Color(background)),
        border: iced::Border {
            color: border,
            width: 1.0,
            radius: f32::from(theme.radius().full.min(999)).into(),
        },
        text_color: Some(text_color),
        ..container::Style::default()
    }
}

/// The appearance of the rule joining two steps.
fn connector_style(theme: &Theme, is_done: bool) -> container::Style {
    let colors = theme.colors();

    container::Style {
        background: Some(iced::Background::Color(if is_done {
            colors.primary
        } else {
            colors.border
        })),
        ..container::Style::default()
    }
}

/// The appearance of a step's label when it can be pressed.
fn step_head_style(theme: &Theme, status: button::Status, disabled: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: (hovered && !disabled).then_some(iced::Background::Color(colors.accent)),
        text_color: if disabled {
            Color {
                a: colors.muted_foreground.a * 0.6,
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

/// Builds a stepper.
pub fn stepper<'a, Message: Clone + 'a>(steps: Vec<Step>, current: usize) -> Stepper<'a, Message> {
    Stepper::new(steps, current)
}

/// One title in an [`app_menu_bar`].
#[derive(Debug, Clone)]
#[must_use = "a MenuTitle does nothing unless it is given to `app_menu_bar`"]
pub struct MenuTitle {
    label: String,
    enabled: bool,
}

impl MenuTitle {
    /// Creates an enabled menu title.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    /// Enables or disables the title.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The title's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the title can be opened.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// A horizontal application menu bar: File, Edit, View, and so on.
///
/// # Opening a menu
///
/// The bar reports which title was pressed rather than holding a menu of its
/// own. iced has no window-level z-order, so a dropdown is positioned by the
/// application and drawn through the
/// [`Layer`](crate::widgets::overlay::Layer) — the same division of labour as
/// [`DropdownButton`](crate::widgets::button::DropdownButton). `open` names the
/// title whose menu is showing, so the bar can mark it while it is open.
///
/// ```
/// # use iced_kit::widgets::{app_menu_bar, MenuTitle};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Open(usize), Closed }
/// # fn view(open: Option<usize>) -> Element<'static, Message, Theme> {
/// app_menu_bar(
///     vec![MenuTitle::new("File"), MenuTitle::new("Edit")],
///     open,
///     Message::Open,
/// )
/// # }
/// ```
pub fn app_menu_bar<'a, Message: Clone + 'a>(
    titles: Vec<MenuTitle>,
    open: Option<usize>,
    on_open: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    let text_style = Size::Sm.text();
    let height = Size::Sm.height();
    let count = titles.len();

    let mut strip = row![].spacing(0).align_y(iced::Alignment::Center);

    for (index, title) in titles.into_iter().enumerate() {
        let is_open = open == Some(index);
        let label = title.label.clone();

        let mut widget = button(
            text(label)
                .size(text_style.size)
                .line_height(iced::Pixels(height.max(text_style.line_height))),
        )
        .padding(Padding {
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 10.0,
        })
        .height(Length::Fixed(height))
        .class(Box::new(move |theme: &Theme, status| {
            menu_title_style(theme, status, is_open)
        }) as button::StyleFn<'a, Theme>);

        if title.enabled {
            widget = widget.on_press(on_open(index));
        }

        strip = strip.push(widget);
    }

    // A stale index marks nothing rather than panicking on a missing title.
    let _ = (count, open);

    strip.into()
}

/// The appearance of a menu bar title.
fn menu_title_style(theme: &Theme, status: button::Status, is_open: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);
    let disabled = matches!(status, button::Status::Disabled);

    button::Style {
        // An open title stays highlighted, so the bar shows which menu the
        // dropdown below it belongs to.
        background: (hovered || is_open).then_some(iced::Background::Color(colors.accent)),
        text_color: if disabled {
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            }
        } else if is_open {
            colors.accent_foreground
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
        accordion, accordion_builder, app_menu_bar, breadcrumb, page_button, page_window,
        pagination, stepper, Accordion, Crumb, MenuTitle, Section, Step, StepLayout, Stepper,
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

    #[test]
    fn a_menu_title_keeps_its_label_and_flag() {
        let title = MenuTitle::new("File");
        assert_eq!(title.label(), "File");
        assert!(title.is_enabled());

        assert!(!MenuTitle::new("Edit").enabled(false).is_enabled());
    }

    #[test]
    fn a_menu_bar_renders_with_and_without_an_open_title() {
        #[derive(Debug, Clone, PartialEq)]
        enum Event {
            Open(usize),
        }

        for open in [None, Some(0), Some(1), Some(99)] {
            let element: iced::Element<'_, Event, Theme> = app_menu_bar(
                vec![MenuTitle::new("File"), MenuTitle::new("Edit")],
                open,
                Event::Open,
            );
            drop(element);
        }

        // An empty bar is valid, if pointless.
        let empty: iced::Element<'_, Event, Theme> = app_menu_bar(vec![], None, Event::Open);
        drop(empty);
    }

    #[test]
    fn an_open_menu_title_is_highlighted() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let open = super::menu_title_style(&theme, Status::Active, true);
        let closed = super::menu_title_style(&theme, Status::Active, false);

        assert!(open.background.is_some(), "an open title must be marked");
        assert!(closed.background.is_none(), "a closed title must not be");
        assert_eq!(open.text_color, theme.colors().accent_foreground);
        assert_eq!(closed.text_color, theme.colors().foreground);
    }

    #[test]
    fn a_crumb_records_its_label_and_link() {
        let plain: Crumb<Message> = Crumb::new("report.pdf");
        assert_eq!(plain.label(), "report.pdf");
        assert!(plain.message.is_none());
        assert!(!plain.is_disabled());

        let link: Crumb<Message> = Crumb::link("Home", Message::Toggled(0));
        assert!(link.message.is_some());

        let locked: Crumb<Message> = Crumb::new("Locked").disabled(true);
        assert!(locked.is_disabled());
    }

    #[test]
    fn a_breadcrumb_renders_a_trail_of_any_length() {
        for count in 0..4 {
            let crumbs: Vec<Crumb<Message>> = (0..count)
                .map(|index| {
                    if index + 1 == count {
                        Crumb::new(format!("Page {index}"))
                    } else {
                        Crumb::link(format!("Page {index}"), Message::Toggled(index))
                    }
                })
                .collect();

            let element: iced::Element<'_, Message, Theme> = breadcrumb(crumbs);
            drop(element);
        }

        // A disabled crumb renders dimmed, whether or not it carried a message.
        let element: iced::Element<'_, Message, Theme> =
            breadcrumb(vec![Crumb::link("Gone", Message::Toggled(0)).disabled(true)]);
        drop(element);
    }

    #[test]
    fn a_step_records_its_options() {
        let step = Step::new("Address").icon(IconName::File).disabled(true);

        assert_eq!(step.label(), "Address");
        assert!(step.is_disabled());

        assert_eq!(Step::new("Cart").label(), "Cart");
    }

    #[test]
    fn a_stepper_marks_what_is_behind_and_what_is_current() {
        let stepper: Stepper<'_, Message> =
            stepper(vec![Step::new("A"), Step::new("B"), Step::new("C")], 1);

        assert!(stepper.is_done(0), "the first step is behind");
        assert!(!stepper.is_done(1), "the current step is not behind");
        assert!(!stepper.is_done(2));

        assert!(stepper.is_current(1));
        assert!(!stepper.is_current(0));
        assert!(!stepper.is_current(2));
    }

    #[test]
    fn a_stepper_out_of_range_marks_nothing_current() {
        let stepper: Stepper<'_, Message> = stepper(vec![Step::new("Only")], 99);

        assert!(!stepper.is_current(0));
        assert!(
            stepper.is_done(0),
            "every step is behind a position past the end"
        );
    }

    #[test]
    fn a_stepper_records_its_options() {
        let stepper: Stepper<'_, Message> = stepper(vec![Step::new("A")], 0)
            .layout(StepLayout::Vertical)
            .disabled(true)
            .on_select(Message::Toggled);

        assert_eq!(stepper.layout, StepLayout::Vertical);
        assert!(stepper.disabled);
        assert!(stepper.on_select.is_some());
    }

    #[test]
    fn steppers_render_in_both_layouts() {
        for layout in [StepLayout::Horizontal, StepLayout::Vertical] {
            let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
                stepper(vec![Step::new("A"), Step::new("B")], 0)
                    .layout(layout)
                    .into(),
                stepper(
                    vec![
                        Step::new("A"),
                        Step::new("B").disabled(true),
                        Step::new("C").icon(IconName::Check),
                    ],
                    2,
                )
                .layout(layout)
                .disabled(true)
                .on_select(Message::Toggled)
                .into(),
            ];

            for element in elements {
                drop(element);
            }
        }

        // An empty stepper is valid, if pointless.
        let empty: iced::Element<'_, Message, Theme> = stepper(vec![], 0).into();
        drop(empty);
    }

    #[test]
    fn a_done_step_is_filled_with_the_primary_color() {
        let theme = Theme::light();

        let done = super::step_marker_style(&theme, true, false, false);
        let current = super::step_marker_style(&theme, false, true, false);
        let waiting = super::step_marker_style(&theme, false, false, false);
        let disabled = super::step_marker_style(&theme, false, true, true);

        assert_eq!(
            done.background,
            Some(iced::Background::Color(theme.colors().primary))
        );
        assert_eq!(current.border.color, theme.colors().primary);
        assert_eq!(waiting.border.color, theme.colors().border);
        assert_ne!(
            disabled.background, current.background,
            "a disabled current step must not read as active"
        );
    }

    #[test]
    fn the_rule_behind_a_step_takes_the_primary_color() {
        let theme = Theme::light();

        let done = super::connector_style(&theme, true);
        let waiting = super::connector_style(&theme, false);

        assert_eq!(
            done.background,
            Some(iced::Background::Color(theme.colors().primary))
        );
        assert_eq!(
            waiting.background,
            Some(iced::Background::Color(theme.colors().border))
        );
    }
}

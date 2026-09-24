//! Small display pieces: description lists, status bars, links, copy buttons
//! and star ratings.
//!
//! Each is a few rows of layout rather than a machine, so they live together:
//! they share the same shape — build from plain data, report one message — and
//! splitting them across five files would hide that they are the same kind of
//! thing.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};

/// A label or a value in a [`description_list`].
///
/// An `Element` is neither `Clone` nor `Debug`, so neither is a text carrying
/// one.
pub enum DescriptionText<'a, Message> {
    /// Plain text, drawn in the row's own face.
    Text(String),
    /// Any element, for a value that is not text.
    Element(Element<'a, Message, Theme>),
}

impl<Message> From<&str> for DescriptionText<'_, Message> {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl<Message> From<String> for DescriptionText<'_, Message> {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl<'a, Message> From<Element<'a, Message, Theme>> for DescriptionText<'a, Message> {
    fn from(value: Element<'a, Message, Theme>) -> Self {
        Self::Element(value)
    }
}

/// One row of a description list.
#[must_use = "a Description does nothing unless it is given to `description_list`"]
pub struct Description<'a, Message> {
    label: DescriptionText<'a, Message>,
    value: DescriptionText<'a, Message>,
    span: usize,
}

impl<'a, Message> Description<'a, Message> {
    /// Creates a row with a label and no value yet.
    pub fn new(label: impl Into<DescriptionText<'a, Message>>) -> Self {
        Self {
            label: label.into(),
            value: DescriptionText::Text(String::new()),
            span: 1,
        }
    }

    /// Sets the row's value.
    pub fn value(mut self, value: impl Into<DescriptionText<'a, Message>>) -> Self {
        self.value = value.into();
        self
    }

    /// Spans several columns of the list's grid.
    pub fn span(mut self, span: usize) -> Self {
        self.span = span.max(1);
        self
    }
}

/// Which way a [`description_list`]'s rows run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DescriptionLayout {
    /// The label sits above its value.
    #[default]
    Vertical,
    /// The label sits before its value, at a fixed width.
    Horizontal,
}

/// A list of label-and-value rows: the "details" panel of a record.
///
/// ```
/// # use iced_kit::widgets::{description_list, Description, DescriptionLayout};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// description_list(vec![
///     Description::new("Name").value("Ada Lovelace"),
///     Description::new("Role").value("Mathematician"),
/// ])
/// .layout(DescriptionLayout::Horizontal)
/// .bordered(true)
/// .into()
/// # }
/// ```
#[must_use = "a DescriptionList does nothing unless it is turned into an Element"]
pub struct DescriptionList<'a, Message> {
    items: Vec<Description<'a, Message>>,
    layout: DescriptionLayout,
    label_width: f32,
    bordered: bool,
    columns: usize,
}

impl<'a, Message: 'a> DescriptionList<'a, Message> {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            layout: DescriptionLayout::default(),
            label_width: 140.0,
            bordered: false,
            columns: 1,
        }
    }

    /// Sets the direction the rows run in.
    pub fn layout(mut self, layout: DescriptionLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the width of the label column in a horizontal list. The default is
    /// 140.
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width.max(20.0);
        self
    }

    /// Draws the list as one bordered card, with a rule between rows.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Sets how many columns of rows the list is divided into. The default is 1.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, 4);
        self
    }

    /// Adds a row.
    pub fn item(mut self, item: Description<'a, Message>) -> Self {
        self.items.push(item);
        self
    }

    /// Adds several rows.
    pub fn items(mut self, items: impl IntoIterator<Item = Description<'a, Message>>) -> Self {
        self.items.extend(items);
        self
    }

    /// The rows this list draws, in order.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list has no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Turns the list into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            layout,
            label_width,
            bordered,
            columns,
        } = self;

        let label_style = Size::Xs.text();
        let value_style = Size::Sm.text();
        let heading = layout == DescriptionLayout::Horizontal;

        // Rows are laid out in a grid of `columns` columns. A row that spans
        // more than one is placed on its own line rather than packed beside
        // others, which is what "span" means for a list of this shape.
        let mut body = column![].spacing(if bordered { 0.0 } else { 12.0 });
        let mut line = row![].spacing(24).width(Length::Fill);
        let mut used = 0;

        for item in items {
            let span = item.span.min(columns);
            let is_wide = span > 1;

            if is_wide && used > 0 {
                body = body.push(line);
                line = row![].spacing(24).width(Length::Fill);
                used = 0;
            }

            let label: Element<'a, Message, Theme> = match item.label {
                DescriptionText::Text(value) => text(value)
                    .size(label_style.size)
                    .line_height(label_style.line_height())
                    .class(Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>)
                    .into(),
                DescriptionText::Element(element) => element,
            };

            let value: Element<'a, Message, Theme> = match item.value {
                DescriptionText::Text(value) => text(value)
                    .size(value_style.size)
                    .line_height(value_style.line_height())
                    .into(),
                DescriptionText::Element(element) => element,
            };

            let pair: Element<'a, Message, Theme> = if heading {
                row![
                    container(label).width(Length::Fixed(label_width)),
                    container(value).width(Length::Fill),
                ]
                .spacing(8)
                .into()
            } else {
                column![label, value].spacing(2).into()
            };

            let cell = if bordered {
                container(pair)
                    .width(Length::Fill)
                    .padding(Padding {
                        top: 10.0,
                        right: 12.0,
                        bottom: 10.0,
                        left: 12.0,
                    })
                    .class(Box::new(if used > 0 { none } else { top_border })
                        as container::StyleFn<'a, Theme>)
            } else {
                container(pair).width(Length::Fill)
            };

            if is_wide {
                body = body.push(cell);
                continue;
            }

            line = line.push(cell);
            used += 1;

            if used == columns {
                body = body.push(line);
                line = row![].spacing(24).width(Length::Fill);
                used = 0;
            }
        }

        if used > 0 {
            body = body.push(line);
        }

        if !bordered {
            return body.into();
        }

        container(body)
            .width(Length::Fill)
            .class(Box::new(|theme: &Theme| container::Style {
                border: iced::Border {
                    color: theme.colors().border,
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                ..container::Style::default()
            }) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: 'a> Default for DescriptionList<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<DescriptionList<'a, Message>> for Element<'a, Message, Theme> {
    fn from(list: DescriptionList<'a, Message>) -> Self {
        list.into_element()
    }
}

/// A rule along the top edge, for a row that follows another.
fn top_border(theme: &Theme) -> container::Style {
    container::Style {
        border: iced::Border {
            color: theme.colors().border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// A row above every other: the bordered list's first row has no rule.
fn none(_theme: &Theme) -> container::Style {
    container::Style::default()
}

/// Builds a description list.
pub fn description_list<'a, Message: 'a>(
    items: impl IntoIterator<Item = Description<'a, Message>>,
) -> DescriptionList<'a, Message> {
    DescriptionList::new().items(items)
}

/// A bar along the bottom of an application: three slots, any content.
///
/// The centre slot is centred on the bar rather than on what is left of it,
/// which is what puts a status message in the middle of the window rather than
/// halfway between two unrelated runs of text.
///
/// ```
/// # use iced_kit::widgets::{status_bar, muted_text};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// status_bar()
///     .left(muted_text("main.rs"))
///     .center(muted_text("Ln 12, Col 4"))
///     .right(muted_text("UTF-8"))
///     .into()
/// # }
/// ```
#[must_use = "a StatusBar does nothing unless it is turned into an Element"]
pub struct StatusBar<'a, Message> {
    left: Option<Element<'a, Message, Theme>>,
    center: Option<Element<'a, Message, Theme>>,
    right: Option<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> StatusBar<'a, Message> {
    /// Creates an empty bar.
    pub fn new() -> Self {
        Self {
            left: None,
            center: None,
            right: None,
        }
    }

    /// Sets the leading content.
    pub fn left(mut self, content: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.left = Some(content.into());
        self
    }

    /// Sets the centred content.
    pub fn center(mut self, content: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.center = Some(content.into());
        self
    }

    /// Sets the trailing content.
    pub fn right(mut self, content: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.right = Some(content.into());
        self
    }

    /// Turns the bar into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            left,
            center,
            right,
        } = self;

        // The two side slots share the space, so the centre stays centred
        // whatever the sides hold — a wide left slot would otherwise push it
        // off the middle.
        let mut leading = row![].spacing(12).align_y(Alignment::Center);
        if let Some(left) = left {
            leading = leading.push(left);
        }

        let mut trailing = row![]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::Shrink);
        if let Some(right) = right {
            trailing = trailing.push(right);
        }

        let mut bar = row![]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .push(container(leading).width(Length::FillPortion(1)));

        if let Some(center) = center {
            bar = bar.push(
                container(center)
                    .width(Length::FillPortion(1))
                    .center_x(Length::Fill),
            );
        } else {
            bar = bar.push(container(iced::widget::Space::new()).width(Length::FillPortion(1)));
        }

        bar = bar.push(container(trailing).width(Length::FillPortion(1)));

        container(bar)
            .width(Length::Fill)
            .padding(Padding {
                top: 6.0,
                right: 12.0,
                bottom: 6.0,
                left: 12.0,
            })
            .class(Box::new(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.colors().muted)),
                border: iced::Border {
                    color: theme.colors().border,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                text_color: Some(theme.colors().muted_foreground),
                ..container::Style::default()
            }) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: 'a> Default for StatusBar<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<StatusBar<'a, Message>> for Element<'a, Message, Theme> {
    fn from(bar: StatusBar<'a, Message>) -> Self {
        bar.into_element()
    }
}

/// Builds an empty status bar.
pub fn status_bar<'a, Message: 'a>() -> StatusBar<'a, Message> {
    StatusBar::new()
}

/// A hyperlink: text that reads as a link and opens something.
///
/// The link reports a message rather than opening a URL itself: iced has no
/// command for launching a browser, and an application that wants to open one
/// owns how it does so. The [`Link::href`] is carried for the application to
/// use as it likes.
///
/// ```
/// # use iced_kit::widgets::{link, Link};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Opened }
/// # fn view() -> Element<'static, Message, Theme> {
/// Link::new("iced")
///     .href("https://iced.rs")
///     .on_press(Message::Opened)
///     .into()
/// # }
/// ```
#[must_use = "a Link does nothing unless it is turned into an Element"]
pub struct Link<'a, Message> {
    label: String,
    href: Option<String>,
    message: Option<Message>,
    disabled: bool,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Link<'a, Message> {
    /// Creates a link with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
            message: None,
            disabled: false,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Sets the address the link stands for.
    ///
    /// The address is data: nothing here opens it.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Reports a press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    /// Disables the link: it renders dimmed and emits nothing.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The link's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The link's address, if it has one.
    #[must_use]
    pub fn href_value(&self) -> Option<&str> {
        self.href.as_deref()
    }

    /// Whether the link can be pressed.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Turns the link into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let text_style = Size::Sm.text();
        let disabled = self.disabled;

        let mut widget = button(
            text(self.label)
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
            Box::new(move |theme: &Theme, status| link_style(theme, status, disabled))
                as button::StyleFn<'a, Theme>,
        );

        if let Some(message) = self.message {
            if !disabled {
                widget = widget.on_press(message);
            }
        }

        widget.into()
    }
}

impl<'a, Message: Clone + 'a> From<Link<'a, Message>> for Element<'a, Message, Theme> {
    fn from(link: Link<'a, Message>) -> Self {
        link.into_element()
    }
}

/// Builds a hyperlink.
pub fn link<'a, Message: Clone + 'a>(label: impl Into<String>) -> Link<'a, Message> {
    Link::new(label)
}

/// The appearance of a link.
fn link_style(theme: &Theme, status: button::Status, disabled: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: None,
        text_color: if disabled {
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            }
        } else if hovered {
            colors.link_hover
        } else {
            colors.link
        },
        border: iced::Border {
            // The underline is drawn as a rule under the text rather than as a
            // border, because a border would ring the whole label.
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// A button that copies a value to the clipboard.
///
/// The value is copied by the application: iced hands a widget a clipboard only
/// while it is handling an event, and a copy button's press is that event, so
/// the component reports the value through [`Clipboard::on_copy`] and the
/// application writes it. That also lets the application show its own "copied"
/// feedback, which is the part a component cannot know.
#[must_use = "a ClipboardButton does nothing unless it is turned into an Element"]
pub struct ClipboardButton<'a, Message> {
    value: String,
    label: Option<String>,
    tooltip: String,
    on_copy: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_copied: Option<Message>,
    disabled: bool,
}

impl<'a, Message: Clone + 'a> ClipboardButton<'a, Message> {
    /// Creates a copy button for `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: None,
            tooltip: "Copy".to_owned(),
            on_copy: None,
            on_copied: None,
            disabled: false,
        }
    }

    /// Draws a label beside the icon.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the tooltip shown on hover.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }

    /// Reports the value to copy, carrying it in the message.
    pub fn on_copy(mut self, on_copy: impl Fn(String) -> Message + 'a) -> Self {
        self.on_copy = Some(Box::new(on_copy));
        self
    }

    /// Reports that a copy happened, without carrying the value.
    pub fn on_copied(mut self, message: Message) -> Self {
        self.on_copied = Some(message);
        self
    }

    /// Disables the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The value the button copies.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Turns the button into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let text_style = Size::Sm.text();
        let disabled = self.disabled;

        let mut content = row![].spacing(6).align_y(Alignment::Center);
        content = content.push(crate::widgets::Icon::new(IconName::Copy).into_element(Size::Sm));

        if let Some(label) = self.label {
            content = content.push(
                text(label)
                    .size(text_style.size)
                    .line_height(text_style.line_height()),
            );
        }

        let mut widget = button(content)
            .padding(Padding {
                top: 2.0,
                right: 6.0,
                bottom: 2.0,
                left: 6.0,
            })
            .class(
                Box::new(move |theme: &Theme, status| clipboard_style(theme, status, disabled))
                    as button::StyleFn<'a, Theme>,
            );

        // A copy button reports the value it holds, so the application can put
        // it on the clipboard: iced only lends a clipboard during an event.
        if !disabled {
            if let Some(on_copy) = self.on_copy {
                widget = widget.on_press(on_copy(self.value.clone()));
            } else if let Some(on_copied) = self.on_copied {
                widget = widget.on_press(on_copied);
            }
        }

        let widget: Element<'a, Message, Theme> = widget.into();

        crate::widgets::tooltip(widget, self.tooltip)
    }
}

impl<'a, Message: Clone + 'a> From<ClipboardButton<'a, Message>> for Element<'a, Message, Theme> {
    fn from(button: ClipboardButton<'a, Message>) -> Self {
        button.into_element()
    }
}

/// Builds a copy button.
pub fn clipboard_button<'a, Message: Clone + 'a>(
    value: impl Into<String>,
) -> ClipboardButton<'a, Message> {
    ClipboardButton::new(value)
}

/// The appearance of a copy button.
fn clipboard_style(theme: &Theme, status: button::Status, disabled: bool) -> button::Style {
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

/// A row of stars showing, and choosing, a rating.
///
/// ```
/// # use iced_kit::widgets::{rating, Rating};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Rated(usize) }
/// # fn view(score: usize) -> Element<'static, Message, Theme> {
/// Rating::new(score).max(5).on_select(Message::Rated).into()
/// # }
/// ```
#[must_use = "a Rating does nothing unless it is turned into an Element"]
pub struct Rating<'a, Message> {
    value: usize,
    max: usize,
    color: Option<Color>,
    disabled: bool,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Rating<'a, Message> {
    /// Creates a rating showing `value`.
    pub fn new(value: usize) -> Self {
        Self {
            value,
            max: 5,
            color: None,
            disabled: false,
            on_select: None,
        }
    }

    /// Sets how many stars there are. The default is 5.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max.clamp(1, 10);
        self
    }

    /// Overrides the star color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Disables the rating: it renders but cannot be changed.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reports a pressed star, carrying the new value.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// The rating's value, clamped to its star count.
    #[must_use]
    pub fn value_clamped(&self) -> usize {
        self.value.min(self.max)
    }

    /// Whether star `index` (zero-based) is filled.
    #[must_use]
    pub fn is_filled(&self, index: usize) -> bool {
        index < self.value_clamped()
    }

    /// Turns the rating into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        // The filled count is read before the fields are moved out.
        let filled = self.value_clamped();

        let Self {
            value: _,
            max,
            color,
            disabled,
            on_select,
        } = self;
        let mut stars = row![].spacing(2).align_y(Alignment::Center);

        for index in 0..max {
            let is_filled = index < filled;
            let star: Element<'a, Message, Theme> =
                crate::widgets::Icon::new(IconName::Star).into_element(Size::Md);

            let mut cell =
                button(
                    container(star).class(Box::new(move |theme: &Theme| container::Style {
                        text_color: Some(if is_filled {
                            color.unwrap_or_else(|| theme.colors().warning)
                        } else {
                            theme.colors().muted_foreground
                        }),
                        ..container::Style::default()
                    }) as container::StyleFn<'a, Theme>),
                )
                .padding(Padding::ZERO)
                .class(Box::new(|_theme: &Theme, status| star_button_style(status))
                    as button::StyleFn<'a, Theme>);

            // A star reports the rating it would set, which is one more than
            // its own index: pressing the first star means one star.
            if !disabled {
                if let Some(on_select) = on_select.as_ref() {
                    cell = cell.on_press(on_select(index + 1));
                }
            }

            stars = stars.push(cell);
        }

        stars.into()
    }
}

impl<'a, Message: Clone + 'a> From<Rating<'a, Message>> for Element<'a, Message, Theme> {
    fn from(rating: Rating<'a, Message>) -> Self {
        rating.into_element()
    }
}

/// Builds a star rating.
pub fn rating<'a, Message: Clone + 'a>(value: usize) -> Rating<'a, Message> {
    Rating::new(value)
}

/// The appearance of one star.
fn star_button_style(status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: None,
        text_color: Color::BLACK,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: !hovered,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clipboard_button, description_list, link, rating, status_bar, ClipboardButton, Description,
        DescriptionLayout, DescriptionList, Link, Rating, StatusBar,
    };
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Copied(String),
        Opened,
        Rated(usize),
    }

    #[test]
    fn a_description_list_records_its_options() {
        let list: DescriptionList<'_, Message> = description_list(vec![
            Description::new("Name").value("Ada"),
            Description::new("Notes").value("Line one").span(2),
        ])
        .layout(DescriptionLayout::Horizontal)
        .label_width(90.0)
        .bordered(true)
        .columns(2);

        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        assert_eq!(list.layout, DescriptionLayout::Horizontal);
        assert_eq!(list.label_width, 90.0);
        assert!(list.bordered);
        assert_eq!(list.columns, 2);
    }

    #[test]
    fn a_description_list_clamps_its_shape() {
        let list: DescriptionList<'_, Message> = DescriptionList::new().columns(0).label_width(1.0);

        assert_eq!(list.columns, 1, "a list needs at least one column");
        assert_eq!(list.label_width, 20.0, "a label needs room to be read");

        let wide: DescriptionList<'_, Message> = DescriptionList::new().columns(99);
        assert_eq!(wide.columns, 4, "four columns is as many as reads");
    }

    #[test]
    fn an_empty_description_list_is_empty() {
        let list: DescriptionList<'_, Message> = DescriptionList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(DescriptionList::<'_, Message>::default().len(), 0);
    }

    #[test]
    fn description_lists_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            DescriptionList::<Message>::new().into(),
            {
                let list: DescriptionList<'_, Message> =
                    description_list(vec![Description::new("Name").value("Ada")]);
                list.into()
            },
            {
                let list: DescriptionList<'_, Message> = description_list(vec![
                    Description::new("A").value("1"),
                    Description::new("B").value("2"),
                ])
                .layout(DescriptionLayout::Horizontal)
                .bordered(true)
                .columns(2);
                list.into()
            },
            {
                // A value can be an element rather than text.
                let value: Element<'_, Message, Theme> = iced::widget::text("element").into();
                let list: DescriptionList<'_, Message> =
                    description_list(vec![Description::new("Badge").value(value)]);
                list.into()
            },
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_status_bar_records_its_slots() {
        let bar: StatusBar<'_, Message> = status_bar()
            .left(iced::widget::text("left"))
            .center(iced::widget::text("centre"))
            .right(iced::widget::text("right"));

        assert!(bar.left.is_some());
        assert!(bar.center.is_some());
        assert!(bar.right.is_some());
    }

    #[test]
    fn status_bars_render_with_any_subset_of_their_slots() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            status_bar::<Message>().into(),
            status_bar().left(iced::widget::text("main.rs")).into(),
            status_bar()
                .center(iced::widget::text("Ln 12"))
                .right(iced::widget::text("UTF-8"))
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_link_carries_its_label_and_address() {
        let link: Link<'_, Message> = Link::new("iced")
            .href("https://iced.rs")
            .on_press(Message::Opened);

        assert_eq!(link.label(), "iced");
        assert_eq!(link.href_value(), Some("https://iced.rs"));
        assert!(link.message.is_some());
        assert!(!link.is_disabled());

        let plain: Link<'_, Message> = Link::new("plain");
        assert_eq!(plain.href_value(), None);
    }

    #[test]
    fn links_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            link::<Message>("iced").into(),
            link::<Message>("iced")
                .href("https://iced.rs")
                .on_press(Message::Opened)
                .into(),
            link::<Message>("gone")
                .on_press(Message::Opened)
                .disabled(true)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_disabled_link_takes_the_dimmer_color() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let active = super::link_style(&theme, Status::Active, false);
        let hovered = super::link_style(&theme, Status::Hovered, false);
        let disabled = super::link_style(&theme, Status::Active, true);

        assert_eq!(active.text_color, theme.colors().link);
        assert_eq!(hovered.text_color, theme.colors().link_hover);
        assert!(disabled.text_color.a < active.text_color.a);
    }

    #[test]
    fn a_clipboard_button_holds_the_value_it_copies() {
        let button: ClipboardButton<'_, Message> =
            clipboard_button("cargo test").label("Copy command");

        assert_eq!(button.value(), "cargo test");
        assert_eq!(button.label.as_deref(), Some("Copy command"));
        assert_eq!(button.tooltip, "Copy");
    }

    #[test]
    fn clipboard_buttons_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            clipboard_button::<Message>("value").into(),
            clipboard_button::<Message>("value")
                .label("Copy")
                .tooltip("Copy to clipboard")
                .on_copy(Message::Copied)
                .into(),
            clipboard_button::<Message>("value")
                .on_copied(Message::Copied(String::new()))
                .disabled(true)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_rating_clamps_its_value_to_its_stars() {
        let three: Rating<'_, Message> = rating(3).max(5);
        assert_eq!(three.value_clamped(), 3);

        let over: Rating<'_, Message> = rating(9).max(5);
        assert_eq!(over.value_clamped(), 5, "a rating cannot exceed its stars");

        let no_stars: Rating<'_, Message> = rating(0).max(0);
        assert_eq!(no_stars.max, 1, "a rating needs at least one star");
    }

    #[test]
    fn a_rating_reports_which_stars_are_filled() {
        let two: Rating<'_, Message> = rating(2).max(5);

        assert!(two.is_filled(0));
        assert!(two.is_filled(1));
        assert!(!two.is_filled(2));
        assert!(!two.is_filled(4));
    }

    #[test]
    fn a_rating_of_zero_fills_nothing() {
        let empty: Rating<'_, Message> = rating(0).max(5);

        assert_eq!(empty.value_clamped(), 0);
        assert!((0..5).all(|index| !empty.is_filled(index)));
    }

    #[test]
    fn ratings_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            rating::<Message>(0).into(),
            rating::<Message>(3).max(5).on_select(Message::Rated).into(),
            rating::<Message>(5)
                .max(10)
                .color(iced::Color::from_rgb8(1, 2, 3))
                .disabled(true)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }
}

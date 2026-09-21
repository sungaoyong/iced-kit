//! Lists and tags.

use crate::theme::{Size, Theme};
use crate::widgets::display::Tone;
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length, Padding};

/// One row of a [`list`].
#[derive(Debug, Clone)]
#[must_use = "a ListItem does nothing unless it is given to `list`"]
pub struct ListItem {
    title: String,
    subtitle: Option<String>,
    trailing: Option<String>,
    enabled: bool,
}

impl ListItem {
    /// Creates an enabled row.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            trailing: None,
            enabled: true,
        }
    }

    /// Adds a secondary line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Adds trailing text, such as a count or a timestamp.
    pub fn trailing(mut self, trailing: impl Into<String>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Enables or disables the row.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the row's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether the row can be selected.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Builds a selectable list.
///
/// `selected` is the index of the highlighted row, or `None`. A disabled row
/// renders dimmed and emits nothing, which is how a list shows an item the user
/// may see but not choose.
///
/// ```
/// # use iced_kit::widgets::{list, ListItem};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Picked(usize) }
/// # fn view(selected: Option<usize>) -> Element<'static, Message, Theme> {
/// list(
///     vec![
///         ListItem::new("Inbox").trailing("12"),
///         ListItem::new("Drafts").subtitle("3 unsent"),
///     ],
///     selected,
///     Message::Picked,
/// )
/// # }
/// ```
pub fn list<'a, Message: Clone + 'a>(
    items: Vec<ListItem>,
    selected: Option<usize>,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    let on_select: std::rc::Rc<dyn Fn(usize) -> Message + 'a> = std::rc::Rc::new(on_select);
    let title_style = Size::Md.text();
    let detail_style = Size::Sm.text();

    let mut rows = column![].spacing(2);

    for (index, item) in items.into_iter().enumerate() {
        let is_selected = selected == Some(index);
        let enabled = item.enabled;

        let mut content = column![text(item.title)
            .size(title_style.size)
            .line_height(title_style.line_height())]
        .spacing(2)
        .width(Length::Fill);

        if let Some(subtitle) = item.subtitle {
            content = content.push(
                text(subtitle)
                    .size(detail_style.size)
                    .line_height(detail_style.line_height())
                    .class(Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>),
            );
        }

        let mut entry = row![content].spacing(12).align_y(iced::Alignment::Center);

        if let Some(trailing) = item.trailing {
            entry = entry.push(text(trailing).size(detail_style.size).class(Box::new(
                |theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                },
            )
                as text::StyleFn<'a, Theme>));
        }

        let mut widget = button(entry.width(Length::Fill))
            .width(Length::Fill)
            .padding(Padding {
                top: 8.0,
                right: 12.0,
                bottom: 8.0,
                left: 12.0,
            })
            .class(Box::new(move |theme: &Theme, status| {
                row_style(theme, status, is_selected, enabled)
            }) as button::StyleFn<'a, Theme>);

        if enabled {
            widget = widget.on_press(std::rc::Rc::clone(&on_select)(index));
        }

        rows = rows.push(widget);
    }

    rows.into()
}

/// The appearance of one list row.
fn row_style(
    theme: &Theme,
    status: button::Status,
    is_selected: bool,
    enabled: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: if is_selected {
            // `accent` and `muted` are the same value in the token set, so a
            // selection tinted with `accent` would be indistinguishable from a
            // hover. The primary color marks the selection unambiguously.
            Some(iced::Background::Color(crate::theme::catalog::blend(
                Color {
                    a: 0.12,
                    ..colors.primary
                },
                colors.background,
            )))
        } else if hovered && enabled {
            Some(iced::Background::Color(colors.muted))
        } else {
            None
        },
        text_color: if !enabled {
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            }
        } else if is_selected {
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

/// Builds a tag: a small labelled chip, optionally removable.
///
/// ```
/// # use iced_kit::widgets::{tag, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Remove }
/// # fn view() -> Element<'static, Message, Theme> {
/// tag("rust", Tone::Primary, Some(Message::Remove))
/// # }
/// ```
pub fn tag<'a, Message: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    tone: Tone,
    on_remove: Option<Message>,
) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    let mut content = row![text(label)
        .size(style.size)
        .line_height(style.line_height())]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    if let Some(message) = on_remove {
        content = content.push(
            button(text("✕").size(style.size - 2.0))
                .padding(Padding::new(0.0))
                .class(Box::new(|theme: &Theme, _status| button::Style {
                    background: None,
                    text_color: theme.colors().muted_foreground,
                    border: iced::Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                }) as button::StyleFn<'a, Theme>)
                .on_press(message),
        );
    }

    container(content)
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .class(Box::new(move |theme: &Theme| container::Style {
            background: Some(iced::Background::Color(tone.surface(theme))),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().full.min(999)).into(),
            },
            text_color: Some(tone.accent(theme)),
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{list, row_style, tag, ListItem};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked(usize),
        Remove,
    }

    #[test]
    fn a_list_renders_with_and_without_a_selection() {
        let items = vec![
            ListItem::new("Inbox").trailing("12"),
            ListItem::new("Drafts").subtitle("3 unsent"),
        ];

        let unselected: iced::Element<'_, Message, Theme> =
            list(items.clone(), None, Message::Picked);
        drop(unselected);

        let selected: iced::Element<'_, Message, Theme> = list(items, Some(1), Message::Picked);
        drop(selected);
    }

    #[test]
    fn an_empty_list_renders() {
        let element: iced::Element<'_, Message, Theme> = list(Vec::new(), None, Message::Picked);
        drop(element);
    }

    #[test]
    fn a_list_ignores_an_out_of_range_selection() {
        let element: iced::Element<'_, Message, Theme> =
            list(vec![ListItem::new("Only")], Some(99), Message::Picked);
        drop(element);
    }

    #[test]
    fn a_disabled_row_does_not_highlight_on_hover() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let disabled = row_style(&theme, Status::Hovered, false, false);
        let enabled = row_style(&theme, Status::Hovered, false, true);

        assert!(enabled.background.is_some(), "an enabled row highlights");
        assert!(disabled.background.is_none(), "a disabled row must not");
    }

    #[test]
    fn a_selected_row_is_visually_distinct_from_a_hovered_one() {
        use iced::widget::button::Status;

        // `accent` and `muted` hold the same value in the token set, so this
        // pins that selection is still marked by a different color.
        for theme in [Theme::light(), Theme::dark()] {
            let selected = row_style(&theme, Status::Active, true, true);
            let hovered = row_style(&theme, Status::Hovered, false, true);

            assert_ne!(
                selected.background, hovered.background,
                "a selected row must not look like a merely hovered one"
            );
        }
    }

    #[test]
    fn a_list_item_keeps_its_fields() {
        let item = ListItem::new("Inbox").subtitle("All mail").trailing("12");
        assert_eq!(item.title(), "Inbox");
        assert!(item.is_enabled());

        let disabled = item.enabled(false);
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn a_tag_renders_with_and_without_a_remove_button() {
        for tone in [Tone::Neutral, Tone::Primary, Tone::Danger] {
            let plain: iced::Element<'_, Message, Theme> = tag("rust", tone, None);
            drop(plain);

            let removable: iced::Element<'_, Message, Theme> =
                tag("rust", tone, Some(Message::Remove));
            drop(removable);
        }
    }
}

//! Tabs.
//!
//! [`tabs`] renders a horizontal strip of selectable labels. It is stateless:
//! the caller owns the selection and receives a message when it changes.

use crate::theme::{Size, Theme};
use iced::widget::{button, row, text};
use iced::{Element, Length, Padding};

/// One tab in a [`tabs`] strip.
#[derive(Debug, Clone)]
#[must_use = "a Tab does nothing unless it is given to `tabs`"]
pub struct Tab {
    label: String,
    enabled: bool,
}

impl Tab {
    /// Creates an enabled tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    /// Enables or disables the tab.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the tab's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Builds a horizontal tab strip.
///
/// `on_select` receives the index of the tab that was clicked. A disabled tab
/// never produces a message.
///
/// ```
/// # use iced_kit::widgets::{tabs, Tab};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Selected(usize) }
/// # fn view(current: usize) -> Element<'static, Message, Theme> {
/// tabs(
///     vec![Tab::new("General"), Tab::new("Advanced")],
///     current,
///     Message::Selected,
/// )
/// .into()
/// # }
/// ```
pub fn tabs<'a, Message: Clone + 'a>(
    tabs: Vec<Tab>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    let text_style = Size::Md.text();
    let height = Size::Md.height() + 4.0;

    let strip =
        tabs.into_iter()
            .enumerate()
            .fold(row![].spacing(0), |row, (index, tab)| {
                let is_selected = index == selected;

                // The line box is the tab's own height, not the text's. A raw
                // iced button lays its content out at the padding origin without
                // centring it, so a text-height line box leaves the label against
                // the top of the tab. Sizing the box to the control is what puts
                // the baseline where the eye expects it.
                let label = text(tab.label)
                    .size(text_style.size)
                    .line_height(iced::Pixels(height.max(text_style.line_height)));

                let mut widget = button(label)
                    .padding(Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 12.0,
                    })
                    .height(Length::Fixed(height))
                    .class(Box::new(move |theme: &Theme, status| {
                        tab_style(theme, status, is_selected)
                    }) as button::StyleFn<'a, Theme>);

                if tab.enabled {
                    widget = widget.on_press(on_select(index));
                }

                row.push(widget)
            });

    strip.into()
}

/// The appearance of a single tab.
fn tab_style(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);
    let disabled = matches!(status, button::Status::Disabled);

    let text_color = if is_selected {
        colors.foreground
    } else if disabled {
        // A disabled tab is dimmer than an ordinary unselected one, so it does
        // not read as merely available.
        iced::Color {
            a: colors.muted_foreground.a * 0.6,
            ..colors.muted_foreground
        }
    } else {
        colors.muted_foreground
    };

    button::Style {
        // A disabled tab must not highlight on hover.
        background: (hovered && !disabled).then_some(iced::Background::Color(colors.accent)),
        text_color,
        border: iced::Border {
            // The selected tab is underlined, which is the affordance that
            // distinguishes tabs from a row of buttons.
            color: if is_selected {
                colors.primary
            } else {
                iced::Color::TRANSPARENT
            },
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{tabs, Tab};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Selected(usize),
    }

    #[test]
    fn a_tab_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("One"), Tab::new("Two"), Tab::new("Three")],
            0,
            Message::Selected,
        );
        drop(element);
    }

    #[test]
    fn a_disabled_tab_still_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("Enabled"), Tab::new("Disabled").enabled(false)],
            0,
            Message::Selected,
        );
        drop(element);
    }

    #[test]
    fn an_empty_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(vec![], 0, Message::Selected);
        drop(element);
    }

    #[test]
    fn a_selection_index_beyond_the_strip_renders_no_selection() {
        // A stale index must not panic; it simply selects nothing.
        let element: iced::Element<'_, Message, Theme> =
            tabs(vec![Tab::new("Only")], 99, Message::Selected);
        drop(element);
    }

    #[test]
    fn tab_style_marks_the_selected_tab() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::tab_style(&theme, Status::Active, true);
        let unselected = super::tab_style(&theme, Status::Active, false);

        assert_ne!(selected.border.color, unselected.border.color);
        assert_eq!(selected.border.color, theme.colors().primary);
    }

    #[test]
    fn tab_labels_are_preserved() {
        let tab = Tab::new("General");
        assert_eq!(tab.label(), "General");
        assert!(tab.enabled);
    }
}

//! The footer of a dialog: its action buttons.

use crate::theme::Size;
use crate::theme::Theme;
use crate::widgets::button as kit_button;
use crate::widgets::button::{Button, Icon};
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};

use super::FOOTER_GAP;

/// The footer section of a dialog.
///
/// The reference's `DialogFooter`: a horizontal row, right-aligned, with a
/// tight gap. Right alignment is the convention the two assembled dialogs
/// follow — the primary action lands where a reader finishes — and
/// [`AlertDialog`](super::AlertDialog) is the exception that centres its
/// buttons instead, because an alert has no single "forward" to point at.
///
/// ```
/// # use iced_kit::widgets::overlay::DialogFooter;
/// # use iced_kit::widgets::button;
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Cancel, Delete }
/// # fn view() -> iced::Element<'static, Message, Theme> {
/// DialogFooter::new()
///     .push(button("Cancel").on_press(Message::Cancel))
///     .push(button("Delete").destructive().on_press(Message::Delete))
///     .into()
/// # }
/// ```
#[must_use = "a DialogFooter does nothing unless it is turned into an Element"]
pub struct DialogFooter<'a, Message> {
    content: Vec<Element<'a, Message, Theme>>,
    spacing: f32,
    centered: bool,
}

impl<'a, Message: 'a> Default for DialogFooter<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> DialogFooter<'a, Message> {
    /// Creates an empty footer, aligned to the end of the row.
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            spacing: FOOTER_GAP,
            centered: false,
        }
    }

    /// Sets the gap between the buttons.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Centres the buttons instead of pushing them to the end.
    ///
    /// This is what an [`AlertDialog`](super::AlertDialog) footer wants: an
    /// alert interrupts rather than advances, so there is no direction for the
    /// eye to be sent in and the buttons read as a balanced pair.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    /// Appends a child.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.content.push(child.into());
        self
    }

    /// Appends children.
    pub fn extend<I>(mut self, children: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Element<'a, Message, Theme>>,
    {
        self.content.extend(children.into_iter().map(Into::into));
        self
    }

    /// Converts the footer into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let content = row(self.content)
            .spacing(self.spacing)
            .align_y(Alignment::Center);

        let container = container(content).width(Length::Fill);

        let container = if self.centered {
            container.center_x(Length::Fill)
        } else {
            container.align_x(Alignment::End)
        };

        container.into()
    }
}

impl<'a, Message: 'a> From<DialogFooter<'a, Message>> for Element<'a, Message, Theme> {
    fn from(footer: DialogFooter<'a, Message>) -> Self {
        footer.into_element()
    }
}

/// Builds a right-aligned row of dialog actions.
///
/// The function form of [`DialogFooter::new`], for a caller that already has
/// its buttons in a `Vec`.
#[must_use]
pub fn dialog_actions<'a, Message: Clone + 'a>(
    actions: Vec<Button<'a, Message>>,
) -> Element<'a, Message, Theme> {
    DialogFooter::new().extend(actions).into_element()
}

/// Builds the close button a dialog draws in its top-right corner.
///
/// The reference puts this button at the top-right of the surface, inset from
/// the padding by [`close_inset`], as a small ghost icon button. Returned as an
/// element so the assembled dialogs can position it and a caller can place its
/// own.
#[must_use]
pub fn dialog_close<'a, Message: Clone + 'a>(message: Message) -> Element<'a, Message, Theme> {
    kit_button::icon_button::<Message>()
        .icon(Icon::new(crate::icons::IconName::X))
        .ghost()
        .size(Size::Sm)
        .on_press(message)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{dialog_actions, dialog_close, DialogFooter};
    use crate::theme::Theme;
    use crate::widgets::button;
    use iced::Element;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Close,
        Confirm,
    }

    #[test]
    fn a_footer_renders_end_aligned_by_default() {
        let element: Element<'_, Message, Theme> = DialogFooter::new()
            .push(button("Cancel").ghost().on_press(Message::Close))
            .push(button("Confirm").primary().on_press(Message::Confirm))
            .into();
        drop(element);
    }

    #[test]
    fn a_centered_footer_renders() {
        let element: Element<'_, Message, Theme> = DialogFooter::new()
            .centered()
            .extend([
                button("Cancel").ghost().on_press(Message::Close),
                button("Confirm").primary().on_press(Message::Confirm),
            ])
            .into();
        drop(element);
    }

    #[test]
    fn an_empty_footer_renders() {
        let element: Element<'_, Message, Theme> = DialogFooter::new().into();
        drop(element);
    }

    #[test]
    fn a_footer_accepts_a_custom_spacing() {
        let footer = DialogFooter::<Message>::new().spacing(16.0);
        assert_eq!(footer.spacing, 16.0);
    }

    #[test]
    fn dialog_actions_render() {
        let element: Element<'_, Message, Theme> = dialog_actions(vec![
            button("Cancel").ghost().on_press(Message::Close),
            button("Confirm").primary().on_press(Message::Confirm),
        ]);
        drop(element);
    }

    #[test]
    fn a_close_button_renders() {
        let element: Element<'_, Message, Theme> = dialog_close(Message::Close);
        drop(element);
    }
}

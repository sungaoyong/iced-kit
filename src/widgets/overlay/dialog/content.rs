//! The body of a dialog.

use crate::theme::Theme;
use iced::widget::{column, container};
use iced::{Element, Length};

use super::DIALOG_GAP;

/// The content section of a dialog.
///
/// The reference's `DialogContent`: a full-width vertical stack for a dialog's
/// body. It takes the full width so its children wrap against the surface
/// rather than shrink to their longest line, and it takes no height of its own
/// so the surface around it shrink-wraps the content instead of stretching to
/// the window.
///
/// ```
/// # use iced_kit::widgets::overlay::DialogContent;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// DialogContent::new()
///     .push(iced::widget::text("Choose a branch to merge."))
///     .into()
/// # }
/// ```
#[must_use = "a DialogContent does nothing unless it is turned into an Element"]
pub struct DialogContent<'a, Message> {
    content: Vec<Element<'a, Message, Theme>>,
    spacing: f32,
}

impl<'a, Message: 'a> Default for DialogContent<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> DialogContent<'a, Message> {
    /// Creates an empty content section.
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            spacing: DIALOG_GAP,
        }
    }

    /// Sets the gap between the children.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
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

    /// Converts the content into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        // Full width, but not full height: a content section that claimed the
        // leftover height would stretch its dialog to the bottom of the window
        // instead of letting the surface shrink-wrap what is in it. A caller
        // that wants the body to fill the space and scroll wraps this in a
        // `scrollable` with a height of its own.
        container(column(self.content).spacing(self.spacing))
            .width(Length::Fill)
            .into()
    }
}

impl<'a, Message: 'a> From<DialogContent<'a, Message>> for Element<'a, Message, Theme> {
    fn from(content: DialogContent<'a, Message>) -> Self {
        content.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::DialogContent;
    use crate::theme::Theme;
    use iced::Element;

    #[test]
    fn a_content_section_renders() {
        let element: Element<'_, (), Theme> =
            DialogContent::new().push(iced::widget::text("Body")).into();
        drop(element);
    }

    #[test]
    fn an_empty_content_section_renders() {
        let element: Element<'_, (), Theme> = DialogContent::new().into();
        drop(element);
    }

    #[test]
    fn a_content_section_accepts_a_custom_spacing() {
        let element: Element<'_, (), Theme> = DialogContent::new()
            .spacing(24.0)
            .extend([iced::widget::text("One"), iced::widget::text("Two")])
            .into();
        drop(element);
    }
}

//! The title of a dialog.

use crate::theme::Theme;
use iced::widget::{text, Text};
use iced::{Element, Font};

use super::{TITLE_LINE_HEIGHT, TITLE_SIZE};

/// Builds a dialog title.
///
/// The reference's `DialogTitle`: semibold at `text_base` (16px), with a 1.25
/// relative line height. It reads larger than the description below it and
/// darker than both, which is what makes the pair scan as a heading and its
/// supporting line rather than as two paragraphs.
///
/// ```
/// # use iced_kit::widgets::overlay::dialog_title;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// dialog_title("Delete project").into()
/// # }
/// ```
#[must_use]
pub fn dialog_title<'a>(content: impl text::IntoFragment<'a>) -> Text<'a, Theme> {
    text(content)
        .size(TITLE_SIZE)
        .line_height(iced::Pixels(TITLE_LINE_HEIGHT))
        .font(Font {
            weight: iced::font::Weight::Semibold,
            ..Font::DEFAULT
        })
}

/// A dialog title as an element, for a header that mixes it with other parts.
#[must_use]
pub fn title_element<'a, Message: 'a>(
    content: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    dialog_title(content).into()
}

#[cfg(test)]
mod tests {
    use super::title_element;
    use crate::theme::Theme;
    use iced::Element;

    #[test]
    fn a_title_renders() {
        let element: Element<'_, (), Theme> = title_element("Delete project");
        drop(element);
    }

    /// The title reads first: it is set larger than the description under it,
    /// so the pair scans as a heading and its supporting line.
    const _TITLE_OUTRANKS_DESCRIPTION: () =
        assert!(super::TITLE_SIZE > super::super::DESCRIPTION_SIZE);
}
